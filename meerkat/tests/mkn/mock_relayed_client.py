import sys
import time

def main():
    print("Starting mock relayed client...")
    
    # Parse arguments
    relay_peer = "mock_relay_peer"
    if "--relay" in sys.argv:
        idx = sys.argv.index("--relay")
        if idx + 1 < len(sys.argv):
            relay_peer = sys.argv[idx + 1]
            
    client_peer = "12D3KooWMockClientPeerID"
    
    # Print original Rust logs format
    print(f"Server listening at: /ip4/127.0.0.1/tcp/9001/p2p/{relay_peer}/p2p-circuit/p2p/{client_peer}")
    print(f"Service URL: /ip4/127.0.0.1/tcp/9001/p2p/{relay_peer}/p2p-circuit/p2p/{client_peer}/client_svc")
    print("Server running, press Ctrl+C to stop...")
    sys.stdout.flush()
    
    # Wait a bit to simulate client activity, then complete successfully
    time.sleep(2)
    print("Mock client completing successfully.")

if __name__ == "__main__":
    main()
