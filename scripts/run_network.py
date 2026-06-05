"""
Meerkat Network Orchestrator
Author: Dustin Juliano

Purpose:
  Orchestrates and tests multi-node, relay, and multi-hop distributed topologies
  for Meerkat. It dynamically parses a network manifest, launches background server nodes,
  captures their runtime Peer IDs and service URLs, wires dependent nodes together, and
  performs a clean shutdown of all background processes when completed.

Usage:
  python3 scripts/run_network.py manifest_file_path
"""

import os
import sys
import argparse
import time
import subprocess
import signal
import re
import shutil
import atexit
from datetime import datetime

# Seconds to wait for process to terminate before sending SIGKILL
TERMINATE_TIMEOUT = 5.0
# Sleep interval (seconds) in the main monitoring loop to avoid high CPU usage
MONITOR_POLL_INTERVAL = 0.25
# Sleep interval (seconds) when polling a server node's log file during startup
STARTUP_POLL_INTERVAL = 0.2
# Maximum time (seconds) to wait for a node to print its Service URL
NODE_STARTUP_TIMEOUT = 10.0
# Maximum time (seconds) to allow client/test runner nodes to run before timing out
# to prevent hangs. Does not affect server nodes
CLIENT_NODE_TIMEOUT = 10.0

# Session ID concept allows preserving logs from different runs of this tool
SESSION_ID = datetime.now().strftime("%Y%m%d_%H%M%S")
# Log paths. This is the root log directory for all sessions
LOG_DIR = os.path.join("tmp", "logs")
# Log path for a specific session
LOG_DIR_SESSION = os.path.join(LOG_DIR, "mkn", SESSION_ID)

def exit_orchestrator(code, processes, reason=None):
    """Logs the orchestrator's exit reason, cleans up child nodes, and exits the script."""
    if not reason:
        if code == 0:
            reason = "Success (all nodes completed successfully)"
        else:
            reason = f"Failure (exited with code {code})"
    print(f"\nOrchestrator exiting: {reason}")
    cleanup_processes(processes)
    sys.exit(code)

def handle_exception(exc_type, exc_value, exc_traceback, processes):
    """Intercepts unhandled runtime exceptions, prints the exit crash details, and triggers cleanup."""
    if issubclass(exc_type, KeyboardInterrupt):
        reason = "Terminated by user (KeyboardInterrupt)"
    else:
        reason = f"Crashed due to unhandled exception: {exc_type.__name__}: {exc_value}"
    print(f"\nOrchestrator exiting: {reason}")
    cleanup_processes(processes)
    sys.__excepthook__(exc_type, exc_value, exc_traceback)

def cleanup_processes(processes):
    """Iterates over and cleanly shuts down all background server and client processes."""
    if not processes:
        return
    print("\nShutting down all Meerkat nodes...")
    for p in processes:
        node_name = p["name"]
        proc = p["proc"]
        status = proc.poll()
        if status is not None:
            print(f"Node '{node_name}' (PID: {proc.pid}) has already exited with code {status}.")
        else:
            print(f"Stopping node '{node_name}' (PID: {proc.pid}) with terminate()...")
            try:
                proc.terminate()
                try:
                    exit_code = proc.wait(timeout=TERMINATE_TIMEOUT)
                    print(f"Node '{node_name}' (PID: {proc.pid}) stopped cleanly (Exit code: {exit_code}).")
                except subprocess.TimeoutExpired:
                    print(f"Warning: Node '{node_name}' (PID: {proc.pid}) did not terminate in time. Killing...")
                    proc.kill()
                    exit_code = proc.wait()
                    print(f"Node '{node_name}' (PID: {proc.pid}) killed (Exit code: {exit_code}).")
            except Exception as e:
                print(f"Error: Failed to stop node '{node_name}' (PID: {proc.pid}): {e}", file=sys.stderr)
    processes.clear()
    print("Cleanup complete.")

# Register signal handlers dynamically for clean exits
def register_signals(processes):
    """Dynamically configures handlers for termination signals supported by the current OS."""
    def signal_handler(sig, frame):
        """Handles incoming OS signals, prints the signal name, cleans up nodes, and exits."""
        try:
            sig_name = signal.Signals(sig).name
        except Exception:
            sig_name = f"Signal {sig}"
        print(f"\nOrchestrator exiting: Terminated by signal: {sig_name}")
        cleanup_processes(processes)
        sys.exit(128 + sig)

    signals = ["SIGINT", "SIGTERM"]
    if sys.platform == "win32":
        signals.append("SIGBREAK")
    else:
        signals.extend(["SIGHUP", "SIGQUIT"])

    for sig_name in signals:
        if hasattr(signal, sig_name):
            try:
                signal.signal(getattr(signal, sig_name), signal_handler)
            except (ValueError, OSError):
                pass

def main():
    """Entry point: registers exit hooks, parses the manifest, spawns nodes, and runs the monitoring loop."""
    # Track processes that need to be cleaned up on exit
    processes = []

    # Register handlers meant to run once when the program starts
    sys.excepthook = lambda t, v, tb: handle_exception(t, v, tb, processes)
    atexit.register(cleanup_processes, processes)
    register_signals(processes)

    # Parse arguments
    arg_parser = argparse.ArgumentParser(
        description="Meerkat Network Orchestrator: orchestrates and tests multi-node, relay, and multi-hop distributed topologies for Meerkat.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Manifest file format:
  Each line defines a node in the format:
    node_name:file_path:port:imports

  - node_name: Unique identifier for the node.
  - file_path: Path to the Meerkat (.mkt) file.
  - port: Port to listen on, or 'client' for a client-only node.
  - imports: (Optional) Comma-separated list of node names this node imports.

Example manifest:
  node1: meerkat/tests/net_orch1.mkt: 9001:
  node2: meerkat/tests/net_orch2.mkt: 9002: node1_a, node1_b
  node3: meerkat/tests/net_orch3.mkt: client: node2
"""
    )
    arg_parser.add_argument(
        "manifest_file_path",
        help="Path to the network manifest file"
    )
    if len(sys.argv) == 1:
        arg_parser.print_help()
        sys.exit(0)

    args = arg_parser.parse_args()
    manifest_path = args.manifest_file_path

    if not os.path.isfile(manifest_path):
        print(f"Error: Manifest file '{manifest_path}' not found.")
        exit_orchestrator(1, processes, f"Manifest file '{manifest_path}' not found")

    # Create session-based log directory with clients and servers subdirs
    clients_log_dir = os.path.join(LOG_DIR_SESSION, "clients")
    servers_log_dir = os.path.join(LOG_DIR_SESSION, "servers")
    os.makedirs(clients_log_dir, exist_ok=True)
    os.makedirs(servers_log_dir, exist_ok=True)

    print("===================================================")
    print("       Starting Meerkat Orchestrated Network       ")
    print("===================================================")
    print(f"Using manifest: {manifest_path}")
    print(f"Logs will be written to: {LOG_DIR_SESSION}/")
    print("Offline/loopback mode is active (--local flag enabled)")
    print("---------------------------------------------------")

    # Read manifest nodes
    nodes = []
    with open(manifest_path, 'r') as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith('#'):
                continue
            parts = [p.strip() for p in line.split(':')]
            if len(parts) < 3:
                continue
            node_name = parts[0]
            file_path = parts[1]
            port = parts[2]
            imports = parts[3] if len(parts) > 3 else ""
            nodes.append((node_name, file_path, port, imports))

    # Track started node URLs
    service_urls = {}

    for node_name, file_path, port, imports in nodes:
        # Resolve imports
        import_flags = []
        if imports:
            for imp in [i.strip() for i in imports.split(',') if i.strip()]:
                resolved_url = service_urls.get(imp)
                if not resolved_url:
                    url_file = os.path.join(servers_log_dir, f"{imp}.url")
                    if os.path.exists(url_file):
                        with open(url_file, 'r') as uf:
                            resolved_url = uf.read().strip()
                
                if not resolved_url:
                    print(f"Error: Node '{node_name}' imports '{imp}', but '{imp}' has not been started yet.")
                    exit_orchestrator(1, processes, f"Import resolution error for node '{node_name}'")

                import_flags.extend(["-i", resolved_url])

        if port.lower() == "client":
            log_file_path = os.path.join(clients_log_dir, f"{node_name}.log")
            # Client Node (runs in background)
            print(f"[{node_name}] Starting client node running '{file_path}'...")
            cmd = ["cargo", "run", "-p", "meerkat", "--", "--local", "-f", file_path] + import_flags
            print(f"Executing: {' '.join(cmd)}")
            print("---------------------------------------------------")
            
            try:
                log_file = open(log_file_path, "w")
                proc = subprocess.Popen(cmd, stdout=log_file, stderr=subprocess.STDOUT, text=True)
                log_file.close()
                processes.append({
                    "name": node_name,
                    "proc": proc,
                    "is_client": True,
                    "log_path": log_file_path
                })
            except Exception as e:
                print(f"[{node_name}] Execution failed: {e}")
                exit_orchestrator(1, processes, f"Client node '{node_name}' failed to execute: {e}")
        else:
            log_file_path = os.path.join(servers_log_dir, f"{node_name}.log")
            # Server Node (runs in background)
            print(f"[{node_name}] Starting server node on port {port} running '{file_path}'...")
            cmd = ["cargo", "run", "-p", "meerkat", "--", "--local", "-s", "-f", file_path, "-p", port] + import_flags
            
            log_file = open(log_file_path, "w")
            proc = subprocess.Popen(cmd, stdout=log_file, stderr=subprocess.STDOUT, text=True)
            log_file.close()
            
            processes.append({
                "name": node_name,
                "proc": proc,
                "is_client": False,
                "log_path": log_file_path
            })

            # Wait for the node to print its Service URL
            print(f"Waiting for '{node_name}' to generate its URL...")
            url_found = False
            svc_url = None
            
            iterations = int(NODE_STARTUP_TIMEOUT / STARTUP_POLL_INTERVAL)
            for _ in range(iterations):
                time.sleep(STARTUP_POLL_INTERVAL)
                if proc.poll() is not None:
                    print(f"Error: Server '{node_name}' crashed during startup. Log output:")
                    with open(log_file_path, "r") as lf:
                        print(lf.read())
                    exit_orchestrator(1, processes, f"Server '{node_name}' crashed during startup")

                if os.path.exists(log_file_path):
                    with open(log_file_path, "r") as lf:
                        content = lf.read()
                        matches = re.findall(r"Service URL:\s+(\S+)", content)
                        if matches:
                            for url in matches:
                                svc_name = url.split('/')[-1]
                                service_urls[svc_name] = url
                            svc_url = matches[0]
                            url_found = True
                            break

            if not url_found:
                print(f"Error: Timeout waiting for server '{node_name}' to start. Log output:")
                with open(log_file_path, "r") as lf:
                    print(lf.read())
                exit_orchestrator(1, processes, f"Timeout waiting for server '{node_name}' to start")

            service_urls[node_name] = svc_url
            
            # Save URL file for team integration
            url_file_path = os.path.join(servers_log_dir, f"{node_name}.url")
            with open(url_file_path, "w") as uf:
                uf.write(svc_url)
                
            print(f"[{node_name}] Started successfully! Service URL: {svc_url}\n")

    # Run monitoring loop
    active_clients = [p for p in processes if p["is_client"]]
    if active_clients:
        print(f"Monitoring running nodes (timeout: {CLIENT_NODE_TIMEOUT}s)...")
    monitor_start_time = time.time()
    
    while active_clients:
        time.sleep(MONITOR_POLL_INTERVAL)
        
        # Check for global node timeout to prevent hangs
        if time.time() - monitor_start_time > CLIENT_NODE_TIMEOUT:
            print(f"\nError: Node execution timed out after {CLIENT_NODE_TIMEOUT} seconds.")
            for p in active_clients:
                print(f"--- Log output for active client '{p['name']}' ---")
                if os.path.exists(p["log_path"]):
                    with open(p["log_path"], "r") as lf:
                        print(lf.read())
            exit_orchestrator(1, processes, f"Timeout after {CLIENT_NODE_TIMEOUT} seconds waiting for clients to complete")
            
        # Check for server crashes
        for p in processes:
            if not p["is_client"]:
                status = p["proc"].poll()
                if status is not None:
                    # Server crashed!
                    print(f"\nError: Server '{p['name']}' (PID: {p['proc'].pid}) exited unexpectedly with code {status}.")
                    print(f"--- Log output for '{p['name']}' ---")
                    if os.path.exists(p["log_path"]):
                        with open(p["log_path"], "r") as lf:
                            print(lf.read())
                    exit_orchestrator(1, processes, f"Server '{p['name']}' exited unexpectedly")

        # Check for client completion/failure
        for p in list(active_clients):
            status = p["proc"].poll()
            if status is not None:
                active_clients.remove(p)
                
                # Sequentially dump the log file content to stdout now that it's complete
                print(f"\n--- Log output for client '{p['name']}' ---")
                if os.path.exists(p["log_path"]):
                    with open(p["log_path"], "r") as lf:
                        print(lf.read())
                print(f"--- End of log output for '{p['name']}' ---\n")
                
                if status == 0:
                    print(f"[{p['name']}] Completed successfully.")
                else:
                    print(f"[{p['name']}] Failed with exit code {status}.")
                    exit_orchestrator(status, processes, f"Client node '{p['name']}' failed")

    # If all nodes finished successfully
    print("\n===================================================")
    print("      All manifest nodes completed successfully     ")
    print("===================================================")
    exit_orchestrator(0, processes, "Success (all nodes completed successfully)")

main()
