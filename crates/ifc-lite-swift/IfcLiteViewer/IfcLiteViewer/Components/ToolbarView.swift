import SwiftUI
import IfcLite

struct ToolbarView: ToolbarContent {
    @EnvironmentObject var viewModel: ViewerViewModel

    var body: some ToolbarContent {
        #if os(macOS)
        // Leading items
        ToolbarItemGroup(placement: .navigation) {
            Button(action: viewModel.openFileDialog) {
                Label("Open", systemImage: "folder")
            }
            .help("Open IFC File (⌘O)")
        }

        // Center items - View controls
        ToolbarItemGroup(placement: .principal) {
            Picker("Tool", selection: .constant("orbit")) {
                Label("Select", systemImage: "cursorarrow").tag("select")
                Label("Orbit", systemImage: "arrow.triangle.2.circlepath").tag("orbit")
                Label("Pan", systemImage: "hand.draw").tag("pan")
            }
            .pickerStyle(.segmented)
            .fixedSize()

            Divider()

            Button(action: { /* TODO: Home view */ }) {
                Label("Home", systemImage: "house")
            }
            .help("Reset to home view")

            Button(action: { /* TODO: Fit all */ }) {
                Label("Fit All", systemImage: "arrow.up.left.and.arrow.down.right")
            }
            .help("Fit all elements in view")

            Divider()

            Button(action: viewModel.showAll) {
                Label("Show All", systemImage: "eye")
            }
            .help("Show all hidden elements")

            Button(action: viewModel.isolateSelection) {
                Label("Isolate", systemImage: "eye.circle")
            }
            .disabled(viewModel.selectedIds.isEmpty)
            .help("Isolate selected elements")
        }

        // Trailing items
        ToolbarItemGroup(placement: .automatic) {
            Toggle(isOn: $viewModel.leftPanelVisible) {
                Label("Hierarchy", systemImage: "sidebar.left")
            }
            .help("Toggle hierarchy panel")

            Toggle(isOn: $viewModel.rightPanelVisible) {
                Label("Properties", systemImage: "sidebar.right")
            }
            .help("Toggle properties panel")

            Divider()

            Button(action: { viewModel.isDarkMode.toggle() }) {
                Label("Theme", systemImage: viewModel.isDarkMode ? "moon.fill" : "sun.max.fill")
            }
            .help("Toggle dark/light mode")
        }

        #else
        // iOS toolbar
        ToolbarItem(placement: .navigationBarLeading) {
            Button(action: { /* TODO: Open file picker */ }) {
                Label("Open", systemImage: "folder")
            }
        }

        ToolbarItemGroup(placement: .navigationBarTrailing) {
            Button(action: viewModel.showAll) {
                Label("Show All", systemImage: "eye")
            }

            Menu {
                Button(action: { viewModel.isDarkMode.toggle() }) {
                    Label(viewModel.isDarkMode ? "Light Mode" : "Dark Mode",
                          systemImage: viewModel.isDarkMode ? "sun.max.fill" : "moon.fill")
                }

                Toggle("Hierarchy", isOn: $viewModel.leftPanelVisible)
                Toggle("Properties", isOn: $viewModel.rightPanelVisible)
            } label: {
                Label("Options", systemImage: "ellipsis.circle")
            }
        }
        #endif
    }
}

// Status bar at the bottom
struct StatusBarView: View {
    @EnvironmentObject var viewModel: ViewerViewModel

    var body: some View {
        HStack {
            // Left: Status
            if viewModel.isLoading {
                ProgressView()
                    .scaleEffect(0.7)
                Text("Loading...")
            } else if let error = viewModel.loadError {
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundColor(.red)
                Text(error)
                    .lineLimit(1)
            } else if viewModel.entities.isEmpty {
                Text("No model loaded")
                    .foregroundColor(.secondary)
            } else {
                Image(systemName: "checkmark.circle.fill")
                    .foregroundColor(.green)
                Text("Ready")
            }

            Spacer()

            // Center: Counts
            if !viewModel.entities.isEmpty {
                Text("\(viewModel.visibleCount)/\(viewModel.entities.count) visible")
                    .foregroundColor(.secondary)

                if !viewModel.selectedIds.isEmpty {
                    Text("•")
                        .foregroundColor(.secondary)
                    Text("\(viewModel.selectedIds.count) selected")
                        .foregroundColor(.accentColor)
                }
            }

            Spacer()

            // Right: Filename
            if let fileName = viewModel.fileName {
                Image(systemName: "doc.fill")
                    .foregroundColor(.secondary)
                Text(fileName)
                    .lineLimit(1)
                    .foregroundColor(.secondary)
            }
        }
        .font(.caption)
        .padding(.horizontal)
        .padding(.vertical, 4)
        .background(Color.secondary.opacity(0.15))
    }
}

#Preview {
    VStack {
        Text("Toolbar Preview")
            .toolbar {
                ToolbarView()
            }
    }
    .environmentObject(ViewerViewModel())
}
