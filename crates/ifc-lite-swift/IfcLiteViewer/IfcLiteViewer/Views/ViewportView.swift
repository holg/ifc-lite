import SwiftUI
import IfcLite

/// Main viewport for 3D rendering
struct ViewportView: View {
    @EnvironmentObject var viewModel: ViewerViewModel

    var body: some View {
        ZStack {
            // Background
            Color(white: 0.1)

            if viewModel.entities.isEmpty && !viewModel.isLoading {
                // Empty state
                VStack(spacing: 20) {
                    Image(systemName: "cube.transparent")
                        .font(.system(size: 64))
                        .foregroundColor(.secondary)

                    Text("3D Viewport")
                        .font(.title2)
                        .fontWeight(.semibold)

                    Text("Load an IFC file to view the 3D model")
                        .foregroundColor(.secondary)

                    #if os(macOS)
                    Button("Open IFC File...") {
                        viewModel.openFileDialog()
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.large)
                    #endif
                }
            } else if !viewModel.meshes.isEmpty {
                // SceneKit 3D view
                SceneKitView()
                    .environmentObject(viewModel)
                    .onAppear {
                        print("DEBUG: SceneKitView appeared")
                    }

                // Overlay with model info
                VStack {
                    HStack {
                        Text("3D View Active")
                            .font(.caption)
                            .foregroundColor(.green)
                            .padding(4)
                            .background(Color.black.opacity(0.5))
                            .cornerRadius(4)
                        Spacer()
                    }
                    .padding()

                    Spacer()
                    HStack {
                        Spacer()
                        VStack(alignment: .trailing, spacing: 4) {
                            Text("\(viewModel.meshes.count) meshes")
                            Text("\(formatVertexCount()) vertices")
                            if let bounds = viewModel.bounds {
                                Text(formatBounds(bounds))
                            }
                            Text("Load time: \(viewModel.loadTimeMs)ms")
                        }
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .padding(8)
                        .background(.ultraThinMaterial)
                        .cornerRadius(8)
                        .padding()
                    }
                }
            } else if viewModel.isLoading {
                // Loading state
                VStack(spacing: 16) {
                    ProgressView()
                        .scaleEffect(1.5)
                    Text("Loading...")
                        .foregroundColor(.secondary)
                }
            }

            // Crosshair in center
            if !viewModel.entities.isEmpty && !viewModel.isLoading {
                CrosshairView()
            }
        }
    }

    private func formatVertexCount() -> String {
        let total = viewModel.meshes.reduce(0) { $0 + $1.positions.count / 3 }
        if total > 1_000_000 {
            return String(format: "%.1fM", Double(total) / 1_000_000)
        } else if total > 1_000 {
            return String(format: "%.1fK", Double(total) / 1_000)
        }
        return "\(total)"
    }

    private func formatBounds(_ bounds: SceneBounds) -> String {
        let width = bounds.maxX - bounds.minX
        let height = bounds.maxY - bounds.minY
        let depth = bounds.maxZ - bounds.minZ
        return String(format: "%.1f × %.1f × %.1f", width, height, depth)
    }
}

struct CrosshairView: View {
    var body: some View {
        ZStack {
            Rectangle()
                .fill(Color.secondary.opacity(0.5))
                .frame(width: 1, height: 20)
            Rectangle()
                .fill(Color.secondary.opacity(0.5))
                .frame(width: 20, height: 1)
        }
    }
}

#Preview {
    ViewportView()
        .environmentObject(ViewerViewModel())
}
