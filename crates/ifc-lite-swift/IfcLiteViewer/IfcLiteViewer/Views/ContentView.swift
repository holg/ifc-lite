import SwiftUI
import UniformTypeIdentifiers
import IfcLite

struct ContentView: View {
    @EnvironmentObject var viewModel: ViewerViewModel

    // The Bevy controller manages the 3D rendering
    @StateObject private var bevyController = BevyControllerWrapper()

    var body: some View {
        NavigationSplitView {
            // Left panel - Hierarchy
            HierarchyPanel()
                .frame(minWidth: 250, idealWidth: 280)
        } detail: {
            // Center - 3D Viewport + Right panel
            HStack(spacing: 0) {
                // 3D Viewport - uses Bevy by default, SceneKit as fallback
                ViewportView(bevyController: bevyController.controller)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)

                // Right panel - Properties
                if viewModel.rightPanelVisible {
                    PropertiesPanel()
                        .frame(width: 280)
                }
            }
        }
        .toolbar {
            ToolbarView()
        }
        #if os(macOS)
        .frame(minWidth: 1024, minHeight: 600)
        #endif
        .preferredColorScheme(viewModel.isDarkMode ? .dark : .light)
        .onDrop(of: [.fileURL], isTargeted: nil) { providers in
            handleDrop(providers: providers)
        }
        .overlay {
            if viewModel.isLoading {
                LoadingOverlay()
            }
        }
        .alert("Error", isPresented: .constant(viewModel.loadError != nil)) {
            Button("OK") {
                viewModel.loadError = nil
            }
        } message: {
            Text(viewModel.loadError ?? "Unknown error")
        }
        .fileImporter(
            isPresented: $viewModel.showingFileImporter,
            allowedContentTypes: [UTType(filenameExtension: "ifc") ?? .data],
            allowsMultipleSelection: false
        ) { result in
            switch result {
            case .success(let urls):
                if let url = urls.first {
                    viewModel.loadFile(url: url)
                }
            case .failure(let error):
                viewModel.loadError = error.localizedDescription
            }
        }
    }

    private func handleDrop(providers: [NSItemProvider]) -> Bool {
        guard let provider = providers.first else { return false }

        provider.loadItem(forTypeIdentifier: "public.file-url", options: nil) { item, error in
            guard let data = item as? Data,
                  let url = URL(dataRepresentation: data, relativeTo: nil),
                  url.pathExtension.lowercased() == "ifc" else { return }

            Task { @MainActor in
                viewModel.loadFile(url: url)
            }
        }
        return true
    }
}

struct LoadingOverlay: View {
    var body: some View {
        ZStack {
            Color.black.opacity(0.5)
            VStack(spacing: 16) {
                ProgressView()
                    .scaleEffect(1.5)
                Text("Loading IFC file...")
                    .font(.headline)
            }
            .padding(32)
            .background(.ultraThinMaterial)
            .cornerRadius(16)
        }
    }
}

/// Wrapper to make BevyViewController an ObservableObject for SwiftUI
class BevyControllerWrapper: ObservableObject {
    let controller = BevyViewController()

    deinit {
        controller.stop()
    }
}

#Preview {
    ContentView()
        .environmentObject(ViewerViewModel())
}
