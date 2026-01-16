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
