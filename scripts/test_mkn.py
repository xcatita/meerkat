import subprocess
import json
import sys
import re
import os
import signal

def run_cmd(args, timeout=30):
    """Runs a subprocess command with a timeout, redirecting stderr to stdout.

    Args:
        args: A list of program arguments to execute.
        timeout: Maximum duration in seconds to wait for execution.

    Returns:
        A tuple of (returncode, output), where:
          - returncode: The exit code of the subprocess, or -1 on timeout.
          - output: The combined stdout and stderr of the process.
    """
    is_windows = sys.platform == "win32"
    kwargs = {}
    
    if is_windows:
        # Create a new process group on Windows
        kwargs["creationflags"] = subprocess.CREATE_NEW_PROCESS_GROUP
    else:
        # Create a new session (process group) on POSIX
        kwargs["start_new_session"] = True

    try:
        proc = subprocess.Popen(
            args, 
            stdout=subprocess.PIPE, 
            stderr=subprocess.STDOUT, 
            text=True, 
            **kwargs
        )
        stdout, _ = proc.communicate(timeout=timeout)
        return proc.returncode, stdout
    except subprocess.TimeoutExpired:
        print(f"\nFAIL: Command timed out after {timeout} seconds: {' '.join(args)}")
        
        # Terminate the entire process tree to prevent orphans
        if is_windows:
            try:
                subprocess.run(
                    ["taskkill", "/F", "/T", "/PID", str(proc.pid)],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL
                )
            except Exception:
                pass
        else:
            try:
                os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
            except ProcessLookupError:
                pass
                
        # Reap the process and capture any remaining partial output
        stdout, _ = proc.communicate()
        
        if isinstance(stdout, bytes):
            stdout = stdout.decode(errors="replace")
        elif stdout is None:
            stdout = ""
            
        return -1, stdout

def run_basic_test():
    """Runs integration test 1 (mkn_basic_topology) to verify a basic orchestrator setup."""
    print("Running Test 1: mkn_basic_topology...")
    code, output = run_cmd([
        sys.executable, "scripts/mkn.py", "meerkat/tests/mkn/test_mkn_basic.json"
    ])
    
    print(output)
    
    if code != 0:
        print("FAIL: Basic topology test exited with code", code)
        return False
        
    # Check that basic_server finished successfully (was shutdown)
    # Check that the logs of basic_server contain "All services online."
    # Since they are printed to stdout, we can search the output.
    if "All services online." not in output:
        print("FAIL: Expected 'All services online.' marker in orchestrator output")
        return False
        
    print("PASS: mkn_basic_topology")
    return True

def run_namespace_split_test():
    """Runs integration test 2 (mkn_namespace_split) to verify three-namespace tracking and relay routing."""
    print("\nRunning Test 2: mkn_namespace_split...")
    code, output = run_cmd([
        sys.executable, "scripts/mkn.py", "meerkat/tests/mkn/test_mkn_relay.json", "--dump-state"
    ])
    
    print(output)
    
    if code != 0:
        print("FAIL: Namespace split test exited with code", code)
        return False
        
    # Extract state dump
    marker_start = "--- STATE DUMP ---"
    marker_end = "--- END STATE DUMP ---"
    if marker_start not in output or marker_end not in output:
        print("FAIL: State dump not found in output")
        return False
        
    state_str = output.split(marker_start)[1].split(marker_end)[0].strip()
    try:
        state = json.loads(state_str)
    except Exception as e:
        print("FAIL: Failed to parse state dump JSON:", e)
        return False
        
    relay = state.get("relay_node")
    client = state.get("relayed_client")
    
    if not relay or not client:
        print("FAIL: relay_node or relayed_client missing from state dump")
        return False
        
    # Check relay local_services
    if "relay_svc" not in relay.get("local_services", {}):
        print("FAIL: relay_svc missing from relay local_services")
        return False
        
    # Check relay relayed_services (this verifies the three namespaces and relay tracking!)
    relayed_services = relay.get("relayed_services", {})
    if "client_svc" not in relayed_services:
        print("FAIL: client_svc missing from relay relayed_services (relay proxy tracking failed)")
        return False
        
    # Check service properties
    client_svc = relayed_services["client_svc"]
    if not client_svc.get("is_relayed"):
        print("FAIL: client_svc is_relayed is false, expected true")
        return False
        
    if client_svc.get("relay_peer_id") != relay.get("peer_id"):
        print(f"FAIL: client_svc relay_peer_id ({client_svc.get('relay_peer_id')}) does not match relay's peer_id ({relay.get('peer_id')})")
        return False
        
    # Check client remote_services
    if "relay_svc" not in client.get("remote_services", {}):
        print("FAIL: relay_svc missing from client remote_services")
        return False
        
    print("PASS: mkn_namespace_split (all 3 namespaces verified)")
    return True

def run_validation_failure_test():
    """Runs integration test 3 (mkn_validation_failure) to check 15 error edge cases in manifest validation."""
    print("\nRunning Test 3: mkn_validation_failure...")
    
    test_cases = [
        ("Invalid port", "test_mkn_invalid_port.json", "cannot specify a port"),
        ("Missing alias", "test_mkn_missing_alias.json", "missing 'alias'"),
        ("Empty nodes list", "test_mkn_empty_nodes.json", "'nodes' list cannot be empty"),
        ("Duplicate alias", "test_mkn_duplicate_alias.json", "Duplicate node alias detected"),
        ("Invalid alias format", "test_mkn_invalid_alias_format.json", "must match alphanumeric/underscore format"),
        ("Missing type", "test_mkn_missing_type.json", "missing required 'type' key"),
        ("Invalid type", "test_mkn_invalid_type.json", "type must be 'server' or 'client'"),
        ("Missing file or cmd", "test_mkn_missing_file_or_cmd.json", "must specify either 'file' or 'cmd'"),
        ("Invalid cmd", "test_mkn_invalid_cmd.json", "'cmd' must be a list of strings"),
        ("Invalid port type", "test_mkn_invalid_port_type.json", "'port' must be an integer"),
        ("Server with relay", "test_mkn_server_relay.json", "cannot specify a relay"),
        ("Invalid relay reference", "test_mkn_invalid_relay.json", "which does not exist in the manifest"),
        ("Invalid imports format", "test_mkn_invalid_imports_format.json", "must use 'alias.service_name' dot-notation"),
        ("Invalid imports reference", "test_mkn_invalid_imports_reference.json", "imports from node 'missing' which does not exist"),
        ("Circular dependency", "test_mkn_circular_dependency.json", "Circular dependency detected in manifest"),
    ]
    
    for name, filename, expected_error in test_cases:
        filepath = f"meerkat/tests/mkn/{filename}"
        code, output = run_cmd([sys.executable, "scripts/mkn.py", filepath])
        
        if code == 0:
            print(f"FAIL: Expected non-zero code for {name}")
            print(f"Actual output: {output.strip()}")
            return False
            
        if expected_error not in output:
            print(f"FAIL: Expected validation error regarding '{expected_error}'")
            print(f"Actual output: {output.strip()}")
            return False
            
        print(f"PASS: {name} check")
        
    print("\nPASS: mkn_validation_failure (All 15 edge cases checked)")
    return True

def run_client_timeout_test():
    """Runs integration tests to verify client startup and execution timeout logic."""
    print("\nRunning Test 4: client startup and execution timeouts...")
    
    # 1. Verify slow client (exceeds startup timeout but within execution timeout) succeeds.
    code, output = run_cmd([
        sys.executable, "scripts/mkn.py", "meerkat/tests/mkn/test_mkn_client_slow.json"
    ])
    print(output)
    if code != 0:
        print("FAIL: Slow client test failed, exited with code", code)
        return False
    print("PASS: Slow client test (bypassed startup timeout successfully)")

    # 2. Verify hanging client (exceeds execution timeout) fails with TimeoutError.
    code, output = run_cmd([
        sys.executable, "scripts/mkn.py", "meerkat/tests/mkn/test_mkn_client_exec_timeout.json"
    ])
    print(output)
    if code == 0:
        print("FAIL: Hanging client test exited with 0, expected timeout failure")
        return False
    if "execution timed out" not in output:
        print("FAIL: Expected client execution timeout error in output")
        return False
    print("PASS: Hanging client test (terminated by execution timeout successfully)")
    
    return True

def main():
    """Main entry point to execute the integration test suite and exit with appropriate code."""
    success = True
    success &= run_basic_test()
    success &= run_namespace_split_test()
    success &= run_validation_failure_test()
    success &= run_client_timeout_test()
    
    if success:
        print("\nALL INTEGRATION TESTS PASSED SUCCESSFULLY! ✓")
        sys.exit(0)
    else:
        print("\nSOME INTEGRATION TESTS FAILED.")
        sys.exit(1)

main()
