import SwiftUI
import IfcLite

struct HierarchyPanel: View {
    @EnvironmentObject var viewModel: ViewerViewModel

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Model Hierarchy")
                    .font(.headline)
                Spacer()
                Button(action: viewModel.expandAll) {
                    Image(systemName: "arrow.up.left.and.arrow.down.right")
                }
                .buttonStyle(.borderless)
                .help("Expand All")

                Button(action: viewModel.collapseAll) {
                    Image(systemName: "arrow.down.right.and.arrow.up.left")
                }
                .buttonStyle(.borderless)
                .help("Collapse All")
            }
            .padding(.horizontal)
            .padding(.vertical, 8)

            // Search bar
            HStack {
                Image(systemName: "magnifyingglass")
                    .foregroundColor(.secondary)
                TextField("Search...", text: $viewModel.searchQuery)
                    .textFieldStyle(.plain)
                if !viewModel.searchQuery.isEmpty {
                    Button(action: { viewModel.searchQuery = "" }) {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundColor(.secondary)
                    }
                    .buttonStyle(.borderless)
                }
            }
            .padding(8)
            .background(Color.secondary.opacity(0.1))
            .cornerRadius(8)
            .padding(.horizontal)
            .padding(.bottom, 8)

            Divider()

            // Entity tree
            if viewModel.entities.isEmpty && !viewModel.isLoading {
                EmptyHierarchyView()
            } else if let tree = viewModel.spatialTree {
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 0) {
                        SpatialNodeView(node: tree, depth: 0)
                    }
                }
            } else {
                // Fallback to flat type-grouped list if no spatial tree
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 0) {
                        ForEach(Array(viewModel.groupedEntities.keys.sorted()), id: \.self) { typeKey in
                            if let entities = viewModel.groupedEntities[typeKey] {
                                TypeGroupView(
                                    typeName: typeKey,
                                    entities: entities,
                                    isExpanded: viewModel.expandedNodes.contains(UInt64(bitPattern: Int64(typeKey.hashValue)))
                                )
                            }
                        }
                    }
                }
            }
        }
        .background(Color(white: 0.15))
    }
}

struct EmptyHierarchyView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "doc.badge.plus")
                .font(.system(size: 48))
                .foregroundColor(.secondary)
            Text("No model loaded")
                .font(.headline)
            Text("Open an IFC file or drag and drop here")
                .font(.subheadline)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

/// Recursive view for spatial hierarchy nodes
struct SpatialNodeView: View {
    @EnvironmentObject var viewModel: ViewerViewModel
    let node: SpatialNode
    let depth: Int

    private var isExpanded: Bool {
        viewModel.expandedNodes.contains(node.id)
    }

    private var isSelected: Bool {
        viewModel.selectedIds.contains(node.id)
    }

    private var isVisible: Bool {
        viewModel.isEntityVisible(node.id)
    }

    private var isElement: Bool {
        node.nodeType == "Element"
    }

    private var matchesSearch: Bool {
        if viewModel.searchQuery.isEmpty {
            return true
        }
        let query = viewModel.searchQuery.lowercased()
        return node.name.lowercased().contains(query) ||
               node.entityType.lowercased().contains(query) ||
               node.children.contains { childMatchesSearch($0, query: query) }
    }

    private func childMatchesSearch(_ node: SpatialNode, query: String) -> Bool {
        node.name.lowercased().contains(query) ||
        node.entityType.lowercased().contains(query) ||
        node.children.contains { childMatchesSearch($0, query: query) }
    }

    var body: some View {
        if matchesSearch {
            VStack(alignment: .leading, spacing: 0) {
                // Node row
                HStack(spacing: 4) {
                    // Expand/collapse toggle
                    if !node.children.isEmpty {
                        Button(action: { viewModel.toggleExpanded(node.id) }) {
                            Image(systemName: isExpanded ? "chevron.down" : "chevron.right")
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .frame(width: 16)
                        }
                        .buttonStyle(.borderless)
                    } else {
                        Spacer()
                            .frame(width: 16)
                    }

                    // Icon
                    Text(iconForNode(node))
                        .frame(width: 20)

                    // Name - tap to select (elements) or expand (spatial)
                    Text(node.name)
                        .lineLimit(1)
                        .foregroundColor(isElement && !node.hasGeometry ? .secondary : .primary)

                    Spacer()

                    // Child count badge (for non-elements)
                    if !node.children.isEmpty && !isElement {
                        Text("\(node.children.count)")
                            .font(.caption2)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(Color.secondary.opacity(0.2))
                            .cornerRadius(6)
                    }

                    // Visibility toggle for elements with geometry
                    if isElement && node.hasGeometry {
                        Button(action: {
                            if isVisible {
                                viewModel.hideEntity(node.id)
                            } else {
                                viewModel.showEntity(node.id)
                            }
                        }) {
                            Image(systemName: isVisible ? "eye" : "eye.slash")
                                .font(.caption)
                                .foregroundColor(isVisible ? .secondary : .red)
                        }
                        .buttonStyle(.borderless)
                    }
                }
                .padding(.leading, CGFloat(depth * 16 + 8))
                .padding(.trailing, 8)
                .padding(.vertical, 6)
                .background(isSelected ? Color.accentColor.opacity(0.2) : Color.clear)
                .contentShape(Rectangle())
                .onTapGesture {
                    if isElement {
                        viewModel.select(node.id)
                    } else {
                        viewModel.toggleExpanded(node.id)
                    }
                }

                // Children (if expanded)
                if isExpanded {
                    ForEach(filteredChildren, id: \.id) { child in
                        SpatialNodeView(node: child, depth: depth + 1)
                    }
                }
            }
        }
    }

    private var filteredChildren: [SpatialNode] {
        if viewModel.searchQuery.isEmpty {
            return node.children
        }
        let query = viewModel.searchQuery.lowercased()
        return node.children.filter { child in
            childMatchesSearch(child, query: query)
        }
    }

    private func iconForNode(_ node: SpatialNode) -> String {
        switch node.nodeType {
        case "Project": return "📋"
        case "Site": return "🌍"
        case "Building": return "🏢"
        case "Storey": return "📐"
        case "Space": return "🚪"
        default: return iconForEntityType(node.entityType)
        }
    }

    private func iconForEntityType(_ type: String) -> String {
        let lower = type.lowercased()
        switch lower {
        case let t where t.contains("wall"): return "🧱"
        case let t where t.contains("slab"), let t where t.contains("floor"): return "⬜"
        case let t where t.contains("roof"): return "🏠"
        case let t where t.contains("door"): return "🚪"
        case let t where t.contains("window"): return "🪟"
        case let t where t.contains("stair"): return "🪜"
        case let t where t.contains("column"): return "🏛️"
        case let t where t.contains("beam"): return "📏"
        case let t where t.contains("furniture"): return "🪑"
        case let t where t.contains("pipe"): return "🔧"
        default: return "🔷"
        }
    }
}

/// Fallback type-grouped view (when no spatial tree)
struct TypeGroupView: View {
    @EnvironmentObject var viewModel: ViewerViewModel
    let typeName: String
    let entities: [EntityInfo]
    let isExpanded: Bool

    private var groupId: UInt64 {
        UInt64(bitPattern: Int64(typeName.hashValue))
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Group header
            Button(action: { viewModel.toggleExpanded(groupId) }) {
                HStack {
                    Image(systemName: isExpanded ? "chevron.down" : "chevron.right")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .frame(width: 16)

                    Image(systemName: iconForType(typeName))
                        .foregroundColor(colorForType(typeName))

                    Text(typeName)
                        .fontWeight(.medium)

                    Spacer()

                    Text("\(entities.count)")
                        .font(.caption)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 2)
                        .background(Color.secondary.opacity(0.2))
                        .cornerRadius(8)
                }
                .padding(.horizontal, 8)
                .padding(.vertical, 6)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .background(Color.secondary.opacity(0.1))

            // Expanded items
            if isExpanded {
                ForEach(entities, id: \.id) { entity in
                    EntityRowView(entity: entity)
                }
            }
        }
    }

    private func iconForType(_ type: String) -> String {
        switch type.lowercased() {
        case let t where t.contains("wall"): return "rectangle.split.3x1"
        case let t where t.contains("slab"), let t where t.contains("floor"): return "square.split.2x1"
        case let t where t.contains("roof"): return "house"
        case let t where t.contains("door"): return "door.left.hand.open"
        case let t where t.contains("window"): return "window.horizontal"
        case let t where t.contains("stair"): return "stairs"
        case let t where t.contains("column"): return "cylinder"
        case let t where t.contains("beam"): return "line.horizontal.3"
        case let t where t.contains("furniture"): return "chair"
        case let t where t.contains("pipe"): return "pipe.and.drop"
        default: return "cube"
        }
    }

    private func colorForType(_ type: String) -> Color {
        switch type.lowercased() {
        case let t where t.contains("wall"): return .orange
        case let t where t.contains("slab"), let t where t.contains("floor"): return .gray
        case let t where t.contains("roof"): return .red
        case let t where t.contains("door"): return .brown
        case let t where t.contains("window"): return .blue
        case let t where t.contains("stair"): return .purple
        case let t where t.contains("column"), let t where t.contains("beam"): return .cyan
        default: return .primary
        }
    }
}

struct EntityRowView: View {
    @EnvironmentObject var viewModel: ViewerViewModel
    let entity: EntityInfo

    private var isSelected: Bool {
        viewModel.selectedIds.contains(entity.id)
    }

    private var isVisible: Bool {
        viewModel.isEntityVisible(entity.id)
    }

    var body: some View {
        HStack {
            // Visibility toggle
            Button(action: {
                if isVisible {
                    viewModel.hideEntity(entity.id)
                } else {
                    viewModel.showEntity(entity.id)
                }
            }) {
                Image(systemName: isVisible ? "eye" : "eye.slash")
                    .foregroundColor(isVisible ? .secondary : .red)
            }
            .buttonStyle(.borderless)

            // Entity name
            Text(entity.name ?? "#\(entity.id)")
                .lineLimit(1)

            Spacer()
        }
        .padding(.horizontal, 8)
        .padding(.leading, 24) // Indent under group
        .padding(.vertical, 4)
        .background(isSelected ? Color.accentColor.opacity(0.2) : Color.clear)
        .contentShape(Rectangle())
        .onTapGesture {
            viewModel.select(entity.id)
        }
    }
}

#Preview {
    HierarchyPanel()
        .environmentObject(ViewerViewModel())
        .frame(width: 280)
}
