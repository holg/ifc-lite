import SwiftUI
import IfcLite
import UniformTypeIdentifiers

/// Main view model for the IFC viewer
@MainActor
class ViewerViewModel: ObservableObject {
    // MARK: - Scene State
    @Published var scene: IfcScene
    @Published var isLoading = false
    @Published var loadError: String?
    @Published var fileName: String?
    @Published var loadTimeMs: UInt64 = 0

    // MARK: - Data
    @Published var entities: [EntityInfo] = []
    @Published var meshes: [MeshData] = []
    @Published var bounds: SceneBounds?
    @Published var spatialTree: SpatialNode?

    // MARK: - Selection
    @Published var selectedIds: Set<UInt64> = []
    @Published var hoveredId: UInt64?

    // MARK: - Visibility
    @Published var hiddenIds: Set<UInt64> = []
    @Published var isolatedIds: Set<UInt64>?
    @Published var storeyFilter: String?

    // MARK: - UI State
    @Published var searchQuery = ""
    @Published var expandedNodes: Set<UInt64> = []
    @Published var leftPanelVisible = true
    @Published var rightPanelVisible = true
    @Published var isDarkMode = true

    // MARK: - Camera Commands
    @Published var zoomToEntityId: UInt64? = nil

    // MARK: - Grouped Entities (for hierarchy)
    var groupedEntities: [String: [EntityInfo]] {
        let filtered = filteredEntities
        return Dictionary(grouping: filtered) { entity in
            String(entity.entityType.dropFirst(3)) // Remove "Ifc" prefix
        }
    }

    var filteredEntities: [EntityInfo] {
        if searchQuery.isEmpty {
            return entities
        }
        let query = searchQuery.lowercased()
        return entities.filter { entity in
            (entity.name?.lowercased().contains(query) ?? false) ||
            entity.entityType.lowercased().contains(query)
        }
    }

    var visibleCount: Int {
        entities.filter { isEntityVisible($0.id) }.count
    }

    // MARK: - Initialization

    init() {
        self.scene = IfcScene()
        initLibrary()
    }

    // MARK: - File Loading

    func openFileDialog() {
        #if os(macOS)
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [UTType(filenameExtension: "ifc")!]
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.message = "Select an IFC file to open"

        if panel.runModal() == .OK, let url = panel.url {
            loadFile(url: url)
        }
        #endif
    }

    func loadFile(url: URL) {
        isLoading = true
        loadError = nil
        fileName = url.lastPathComponent

        Task {
            do {
                let result = try scene.loadFile(path: url.path)
                await MainActor.run {
                    self.entities = result.entities
                    self.meshes = result.meshes
                    self.bounds = result.bounds
                    self.spatialTree = result.spatialTree
                    self.loadTimeMs = result.loadTimeMs
                    self.isLoading = false

                    // Reset state
                    self.selectedIds.removeAll()
                    self.hiddenIds.removeAll()
                    self.isolatedIds = nil
                    self.storeyFilter = nil
                    self.expandedNodes.removeAll()

                    // Auto-expand root and first level of spatial tree
                    if let tree = self.spatialTree {
                        self.expandedNodes.insert(tree.id)
                        for child in tree.children {
                            self.expandedNodes.insert(child.id)
                        }
                    } else {
                        // Fallback: expand type groups
                        for key in self.groupedEntities.keys {
                            self.expandedNodes.insert(UInt64(bitPattern: Int64(key.hashValue)))
                        }
                    }
                }
            } catch {
                await MainActor.run {
                    self.loadError = error.localizedDescription
                    self.isLoading = false
                }
            }
        }
    }

    func loadFileFromData(_ data: Data, fileName: String) {
        isLoading = true
        loadError = nil
        self.fileName = fileName

        Task {
            do {
                let result = try scene.loadBytes(data: data)
                await MainActor.run {
                    self.entities = result.entities
                    self.meshes = result.meshes
                    self.bounds = result.bounds
                    self.spatialTree = result.spatialTree
                    self.loadTimeMs = result.loadTimeMs
                    self.isLoading = false

                    // Reset state
                    self.selectedIds.removeAll()
                    self.hiddenIds.removeAll()
                    self.isolatedIds = nil
                    self.expandedNodes.removeAll()

                    // Auto-expand root and first level of spatial tree
                    if let tree = self.spatialTree {
                        self.expandedNodes.insert(tree.id)
                        for child in tree.children {
                            self.expandedNodes.insert(child.id)
                        }
                    } else {
                        // Fallback: expand type groups
                        for key in self.groupedEntities.keys {
                            self.expandedNodes.insert(UInt64(bitPattern: Int64(key.hashValue)))
                        }
                    }
                }
            } catch {
                await MainActor.run {
                    self.loadError = error.localizedDescription
                    self.isLoading = false
                }
            }
        }
    }

    // MARK: - Selection

    func select(_ id: UInt64) {
        selectedIds = [id]
        scene.select(entityId: id)
    }

    func toggleSelection(_ id: UInt64) {
        if selectedIds.contains(id) {
            selectedIds.remove(id)
        } else {
            selectedIds.insert(id)
        }
        scene.toggleSelection(entityId: id)
    }

    func clearSelection() {
        selectedIds.removeAll()
        scene.clearSelection()
    }

    var selectedEntity: EntityInfo? {
        guard let id = selectedIds.first else { return nil }
        return entities.first { $0.id == id }
    }

    // MARK: - Visibility

    func hideEntity(_ id: UInt64) {
        hiddenIds.insert(id)
        scene.hideEntity(entityId: id)
    }

    func showEntity(_ id: UInt64) {
        hiddenIds.remove(id)
        scene.showEntity(entityId: id)
    }

    func isolateEntity(_ id: UInt64) {
        isolatedIds = [id]
        scene.isolateEntity(entityId: id)
    }

    func isolateSelection() {
        guard !selectedIds.isEmpty else { return }
        isolatedIds = selectedIds
        scene.isolateEntities(entityIds: Array(selectedIds))
    }

    func showAll() {
        hiddenIds.removeAll()
        isolatedIds = nil
        scene.showAll()
    }

    func isEntityVisible(_ id: UInt64) -> Bool {
        if hiddenIds.contains(id) { return false }
        if let isolated = isolatedIds, !isolated.contains(id) { return false }
        return true
    }

    // MARK: - Camera

    func zoomToEntity(_ id: UInt64) {
        zoomToEntityId = id
    }

    // MARK: - Tree Navigation

    func toggleExpanded(_ id: UInt64) {
        if expandedNodes.contains(id) {
            expandedNodes.remove(id)
        } else {
            expandedNodes.insert(id)
        }
    }

    func expandAll() {
        if let tree = spatialTree {
            // Recursively collect all node IDs from spatial tree
            collectAllNodeIds(tree, into: &expandedNodes)
        } else {
            // Fallback: expand type groups
            for key in groupedEntities.keys {
                expandedNodes.insert(UInt64(bitPattern: Int64(key.hashValue)))
            }
        }
    }

    func collapseAll() {
        expandedNodes.removeAll()
        // Keep root expanded
        if let tree = spatialTree {
            expandedNodes.insert(tree.id)
        }
    }

    /// Recursively collect all node IDs from a spatial tree
    private func collectAllNodeIds(_ node: SpatialNode, into ids: inout Set<UInt64>) {
        ids.insert(node.id)
        for child in node.children {
            collectAllNodeIds(child, into: &ids)
        }
    }
}
