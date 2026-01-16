import SwiftUI
import IfcLite

@main
struct IfcLiteViewerApp: App {
    @StateObject private var viewModel = ViewerViewModel()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(viewModel)
                .onOpenURL { url in
                    // Handle files opened from Files app or other apps
                    if url.pathExtension.lowercased() == "ifc" {
                        viewModel.loadFile(url: url)
                    }
                }
        }
        #if os(macOS)
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("Open IFC File...") {
                    viewModel.openFileDialog()
                }
                .keyboardShortcut("o", modifiers: .command)
            }
        }
        #endif
    }
}
