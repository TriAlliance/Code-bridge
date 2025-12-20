// Peers View

import SwiftUI

struct PeersView: View {
    @EnvironmentObject var bridgeManager: BridgeManager

    var body: some View {
        VStack(spacing: 0) {
            // Toolbar
            HStack {
                Button(action: { bridgeManager.discoverPeers() }) {
                    Label("Discover", systemImage: "magnifyingglass")
                }
                .buttonStyle(.bordered)

                Spacer()

                Text("Local ID: \(bridgeManager.deviceId)")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            .padding()

            Divider()

            if bridgeManager.peers.isEmpty {
                VStack(spacing: 16) {
                    Image(systemName: "network")
                        .font(.system(size: 48))
                        .foregroundColor(.secondary)
                    Text("No peers connected")
                        .font(.title2)
                    Text("Click Discover to find peers on your network")
                        .foregroundColor(.secondary)
                    Button("Discover Peers") {
                        bridgeManager.discoverPeers()
                    }
                    .buttonStyle(.borderedProminent)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List(bridgeManager.peers) { peer in
                    PeerRow(peer: peer)
                        .contextMenu {
                            Button("Sync with Peer") { }
                            Divider()
                            Button("Remove", role: .destructive) {
                                bridgeManager.removePeer(peer)
                            }
                        }
                }
            }
        }
        .navigationTitle("Peers")
    }
}

struct PeerRow: View {
    let peer: Peer

    var body: some View {
        HStack {
            Circle()
                .fill(peer.isConnected ? Color.green : Color.gray)
                .frame(width: 8, height: 8)

            VStack(alignment: .leading) {
                Text(peer.name)
                    .fontWeight(.medium)
                Text(peer.id.prefix(16) + "...")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            Spacer()

            Text(peer.isConnected ? "Connected" : "Offline")
                .font(.caption)
                .foregroundColor(peer.isConnected ? .green : .secondary)
        }
        .padding(.vertical, 4)
    }
}
