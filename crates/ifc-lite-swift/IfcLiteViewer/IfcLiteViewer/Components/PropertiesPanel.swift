import SwiftUI
import IfcLite

struct PropertiesPanel: View {
    @EnvironmentObject var viewModel: ViewerViewModel

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Properties")
                    .font(.headline)
                Spacer()
            }
            .padding(.horizontal)
            .padding(.vertical, 8)

            Divider()

            if let entity = viewModel.selectedEntity {
                ScrollView {
                    VStack(alignment: .leading, spacing: 16) {
                        // Entity Info Section
                        PropertySection(title: "Entity Info") {
                            PropertyRow(label: "Type", value: entity.entityType)
                            if let name = entity.name {
                                PropertyRow(label: "Name", value: name)
                            }
                            PropertyRow(label: "ID", value: "#\(entity.id)")
                            if let globalId = entity.globalId {
                                PropertyRow(label: "GlobalId", value: globalId, copyable: true)
                            }
                            if let storey = entity.storey {
                                PropertyRow(label: "Storey", value: storey)
                            }
                            if let elevation = entity.storeyElevation {
                                PropertyRow(label: "Elevation", value: String(format: "%.2f m", elevation))
                            }
                        }

                        // Actions Section
                        PropertySection(title: "Actions") {
                            ActionButtonsView(entityId: entity.id)
                        }

                        // Properties Section
                        let propertySets = viewModel.scene.getProperties(entityId: entity.id)
                        ForEach(Array(propertySets.enumerated()), id: \.offset) { index, pset in
                            PropertySection(title: pset.name) {
                                ForEach(Array(pset.properties.enumerated()), id: \.offset) { pIndex, prop in
                                    PropertyRow(
                                        label: prop.name,
                                        value: prop.unit != nil ? "\(prop.value) \(prop.unit!)" : prop.value
                                    )
                                }
                            }
                        }
                    }
                    .padding()
                }
            } else if viewModel.selectedIds.count > 1 {
                MultiSelectionView()
            } else {
                NoSelectionView()
            }
        }
        .background(Color(white: 0.15))
    }
}

struct PropertySection<Content: View>: View {
    let title: String
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.subheadline)
                .fontWeight(.semibold)
                .foregroundColor(.secondary)

            VStack(alignment: .leading, spacing: 4) {
                content
            }
            .padding(12)
            .background(Color.secondary.opacity(0.1))
            .cornerRadius(8)
        }
    }
}

struct PropertyRow: View {
    let label: String
    let value: String
    var copyable: Bool = false

    var body: some View {
        HStack {
            Text(label)
                .foregroundColor(.secondary)
            Spacer()
            Text(value)
                .lineLimit(1)
            if copyable {
                Button(action: copyToClipboard) {
                    Image(systemName: "doc.on.doc")
                        .font(.caption)
                }
                .buttonStyle(.borderless)
            }
        }
        .font(.caption)
    }

    private func copyToClipboard() {
        #if os(macOS)
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(value, forType: .string)
        #else
        UIPasteboard.general.string = value
        #endif
    }
}

struct ActionButtonsView: View {
    @EnvironmentObject var viewModel: ViewerViewModel
    let entityId: UInt64

    var body: some View {
        VStack(spacing: 8) {
            HStack(spacing: 8) {
                ActionButton(title: "Zoom to", icon: "magnifyingglass") {
                    viewModel.zoomToEntity(entityId)
                }

                ActionButton(title: "Isolate", icon: "eye.circle") {
                    viewModel.isolateEntity(entityId)
                }
            }

            HStack(spacing: 8) {
                ActionButton(title: "Hide", icon: "eye.slash") {
                    viewModel.hideEntity(entityId)
                }

                ActionButton(title: "Select Similar", icon: "doc.on.doc") {
                    selectSimilar()
                }
            }
        }
    }

    private func selectSimilar() {
        guard let entity = viewModel.entities.first(where: { $0.id == entityId }) else { return }
        let similarIds = viewModel.entities
            .filter { $0.entityType == entity.entityType }
            .map { $0.id }
        for id in similarIds {
            viewModel.selectedIds.insert(id)
        }
    }
}

struct ActionButton: View {
    let title: String
    let icon: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack {
                Image(systemName: icon)
                Text(title)
                    .font(.caption)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 8)
            .background(Color.secondary.opacity(0.15))
            .cornerRadius(6)
        }
        .buttonStyle(.plain)
    }
}

struct NoSelectionView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "cursorarrow.click")
                .font(.system(size: 36))
                .foregroundColor(.secondary)
            Text("No Selection")
                .font(.headline)
            Text("Click on an element in the 3D view or hierarchy to see its properties.")
                .font(.subheadline)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

struct MultiSelectionView: View {
    @EnvironmentObject var viewModel: ViewerViewModel

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "checkmark.circle.fill")
                .font(.system(size: 36))
                .foregroundColor(.accentColor)

            Text("\(viewModel.selectedIds.count) items selected")
                .font(.headline)

            VStack(spacing: 8) {
                Button("Isolate All") {
                    viewModel.isolateSelection()
                }
                .buttonStyle(.borderedProminent)

                Button("Hide All") {
                    for id in viewModel.selectedIds {
                        viewModel.hideEntity(id)
                    }
                }
                .buttonStyle(.bordered)

                Button("Clear Selection") {
                    viewModel.clearSelection()
                }
                .buttonStyle(.bordered)
            }
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

#Preview {
    PropertiesPanel()
        .environmentObject(ViewerViewModel())
        .frame(width: 280)
}
