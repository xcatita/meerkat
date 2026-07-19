# Persistent node identity (`--identity`)

By default, a Meerkat server generates a new random keypair each time it starts,
so its Peer ID changes on every restart. The `--identity` option makes the Peer
ID stable across restarts by persisting the keypair to disk. A stable Peer ID
means a web page (or any client) can embed a fixed server address.

## Usage

    meerkat -f service.mkt --server --identity path/to/identity.key

If the file at the given path exists, the keypair is loaded from it and the
server reuses the same Peer ID. If the file does not exist, Meerkat generates a
new ed25519 keypair, saves it to that path, and uses it. Later runs with the
same path reuse that keypair.

Omit `--identity` to keep the default behavior of a fresh random identity on
each start.

## Generating a keypair

There is no separate generation step. Run the server once with `--identity`
pointing at a path that does not yet exist, and Meerkat creates the keypair for
you:

    meerkat -f service.mkt --server --identity node.key

The first run creates `node.key`; subsequent runs load it back, keeping the Peer
ID the same.

## The key file

The file stores the keypair in libp2p's protobuf encoding. It contains a
private key, so on Unix it is created atomically with owner-only (`0600`)
permissions. Keep it secret and do not commit it to version control.
