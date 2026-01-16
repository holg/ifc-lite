import SwiftUI
import IfcLite

@main
struct IfcLiteViewerApp: App {
    @StateObject private var viewModel = ViewerViewModel()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(viewModel)
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
