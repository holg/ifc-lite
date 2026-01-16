import SwiftUI
import SceneKit
import IfcLite

#if os(macOS)
typealias ViewRepresentable = NSViewRepresentable
#else
typealias ViewRepresentable = UIViewRepresentable
#endif

/// SceneKit-based 3D view for rendering IFC meshes
struct SceneKitView: ViewRepresentable {
    @EnvironmentObject var viewModel: ViewerViewModel

    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }

    #if os(macOS)
    func makeNSView(context: Context) -> SCNView {
        let scnView = SCNView()
        setupSceneView(scnView, context: context)
        return scnView
    }

    func updateNSView(_ scnView: SCNView, context: Context) {
        context.coordinator.updateScene(scnView: scnView, meshes: viewModel.meshes, bounds: viewModel.bounds)
        context.coordinator.updateSelection(scnView: scnView, selectedIds: viewModel.selectedIds)
        context.coordinator.updateVisibility(scnView: scnView, hiddenIds: viewModel.hiddenIds)

        // Handle zoom to entity
        if let entityId = viewModel.zoomToEntityId {
            context.coordinator.zoomToEntity(scnView: scnView, entityId: entityId)
            DispatchQueue.main.async {
                self.viewModel.zoomToEntityId = nil
            }
        }
    }
    #else
    func makeUIView(context: Context) -> SCNView {
        let scnView = SCNView()
        setupSceneView(scnView, context: context)
        return scnView
    }

    func updateUIView(_ scnView: SCNView, context: Context) {
        context.coordinator.updateScene(scnView: scnView, meshes: viewModel.meshes, bounds: viewModel.bounds)
        context.coordinator.updateSelection(scnView: scnView, selectedIds: viewModel.selectedIds)
        context.coordinator.updateVisibility(scnView: scnView, hiddenIds: viewModel.hiddenIds)

        // Handle zoom to entity
        if let entityId = viewModel.zoomToEntityId {
            context.coordinator.zoomToEntity(scnView: scnView, entityId: entityId)
            DispatchQueue.main.async {
                self.viewModel.zoomToEntityId = nil
            }
        }
    }
    #endif

    private func setupSceneView(_ scnView: SCNView, context: Context) {
        scnView.scene = SCNScene()
        scnView.backgroundColor = NSColor(white: 0.1, alpha: 1.0)
        scnView.allowsCameraControl = true
        scnView.autoenablesDefaultLighting = false
        scnView.showsStatistics = false

        // Add ambient light
        let ambientLight = SCNNode()
        ambientLight.light = SCNLight()
        ambientLight.light?.type = .ambient
        ambientLight.light?.color = NSColor(white: 0.4, alpha: 1.0)
        scnView.scene?.rootNode.addChildNode(ambientLight)

        // Add directional light (sun)
        let sunLight = SCNNode()
        sunLight.light = SCNLight()
        sunLight.light?.type = .directional
        sunLight.light?.color = NSColor(white: 0.8, alpha: 1.0)
        sunLight.light?.castsShadow = true
        sunLight.eulerAngles = SCNVector3(-Float.pi / 4, Float.pi / 4, 0)
        scnView.scene?.rootNode.addChildNode(sunLight)

        // Add camera
        let cameraNode = SCNNode()
        cameraNode.camera = SCNCamera()
        cameraNode.camera?.zNear = 0.1
        cameraNode.camera?.zFar = 10000
        cameraNode.position = SCNVector3(50, 50, 50)
        cameraNode.look(at: SCNVector3(0, 0, 0))
        scnView.scene?.rootNode.addChildNode(cameraNode)
        scnView.pointOfView = cameraNode

        // Add ground grid
        let gridNode = createGridNode()
        scnView.scene?.rootNode.addChildNode(gridNode)

        // Set delegate for tap handling
        let tapGesture = NSClickGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.handleTap(_:)))
        scnView.addGestureRecognizer(tapGesture)

        context.coordinator.sceneView = scnView
    }

    private func createGridNode() -> SCNNode {
        let gridNode = SCNNode()
        let gridSize: Float = 100
        let gridStep: Float = 10

        for i in stride(from: -gridSize, through: gridSize, by: gridStep) {
            // X-axis lines
            let xGeometry = SCNCylinder(radius: 0.02, height: CGFloat(gridSize * 2))
            xGeometry.firstMaterial?.diffuse.contents = NSColor(white: 0.3, alpha: 1.0)
            let xNode = SCNNode(geometry: xGeometry)
            xNode.eulerAngles = SCNVector3(0, 0, Float.pi / 2)
            xNode.position = SCNVector3(0, 0, i)
            gridNode.addChildNode(xNode)

            // Z-axis lines
            let zGeometry = SCNCylinder(radius: 0.02, height: CGFloat(gridSize * 2))
            zGeometry.firstMaterial?.diffuse.contents = NSColor(white: 0.3, alpha: 1.0)
            let zNode = SCNNode(geometry: zGeometry)
            zNode.eulerAngles = SCNVector3(Float.pi / 2, 0, 0)
            zNode.position = SCNVector3(i, 0, 0)
            gridNode.addChildNode(zNode)
        }

        return gridNode
    }

    class Coordinator: NSObject {
        var parent: SceneKitView
        weak var sceneView: SCNView?
        private var meshNodes: [UInt64: SCNNode] = [:]
        private var lastMeshCount = 0

        init(_ parent: SceneKitView) {
            self.parent = parent
        }

        func updateScene(scnView: SCNView, meshes: [MeshData], bounds: SceneBounds?) {
            guard meshes.count != lastMeshCount else { return }
            lastMeshCount = meshes.count

            // Remove old mesh nodes
            for (_, node) in meshNodes {
                node.removeFromParentNode()
            }
            meshNodes.removeAll()

            guard let scene = scnView.scene else { return }

            // Create mesh nodes
            for mesh in meshes {
                if let node = createMeshNode(from: mesh) {
                    scene.rootNode.addChildNode(node)
                    meshNodes[mesh.entityId] = node
                }
            }

            // Fit camera to bounds
            if let bounds = bounds {
                fitCameraToBounds(scnView: scnView, bounds: bounds)
            }
        }

        func updateSelection(scnView: SCNView, selectedIds: Set<UInt64>) {
            for (id, node) in meshNodes {
                if selectedIds.contains(id) {
                    // Highlight selected
                    node.geometry?.firstMaterial?.emission.contents = NSColor.orange.withAlphaComponent(0.3)
                } else {
                    node.geometry?.firstMaterial?.emission.contents = NSColor.black
                }
            }
        }

        func updateVisibility(scnView: SCNView, hiddenIds: Set<UInt64>) {
            for (id, node) in meshNodes {
                node.isHidden = hiddenIds.contains(id)
            }
        }

        private func createMeshNode(from mesh: MeshData) -> SCNNode? {
            let vertexCount = mesh.positions.count / 3
            let normalCount = mesh.normals.count / 3
            let indexCount = mesh.indices.count

            // Validate mesh has sufficient data
            guard vertexCount >= 3,
                  indexCount >= 3,
                  !mesh.positions.isEmpty else {
                return nil
            }

            // Create vertex source
            let vertexData = Data(bytes: mesh.positions, count: mesh.positions.count * MemoryLayout<Float>.size)
            let vertexSource = SCNGeometrySource(
                data: vertexData,
                semantic: .vertex,
                vectorCount: vertexCount,
                usesFloatComponents: true,
                componentsPerVector: 3,
                bytesPerComponent: MemoryLayout<Float>.size,
                dataOffset: 0,
                dataStride: MemoryLayout<Float>.size * 3
            )

            // Create geometry sources array
            var sources: [SCNGeometrySource] = [vertexSource]

            // Add normals if valid
            if normalCount == vertexCount && !mesh.normals.isEmpty {
                let normalData = Data(bytes: mesh.normals, count: mesh.normals.count * MemoryLayout<Float>.size)
                let normalSource = SCNGeometrySource(
                    data: normalData,
                    semantic: .normal,
                    vectorCount: normalCount,
                    usesFloatComponents: true,
                    componentsPerVector: 3,
                    bytesPerComponent: MemoryLayout<Float>.size,
                    dataOffset: 0,
                    dataStride: MemoryLayout<Float>.size * 3
                )
                sources.append(normalSource)
            }

            // Create index element
            let indexData = Data(bytes: mesh.indices, count: indexCount * MemoryLayout<UInt32>.size)
            let element = SCNGeometryElement(
                data: indexData,
                primitiveType: .triangles,
                primitiveCount: indexCount / 3,
                bytesPerIndex: MemoryLayout<UInt32>.size
            )

            // Create geometry
            let geometry = SCNGeometry(sources: sources, elements: [element])

            // Create material
            let material = SCNMaterial()
            if mesh.color.count >= 4 {
                material.diffuse.contents = NSColor(
                    red: CGFloat(mesh.color[0]),
                    green: CGFloat(mesh.color[1]),
                    blue: CGFloat(mesh.color[2]),
                    alpha: CGFloat(mesh.color[3])
                )
                if mesh.color[3] < 1.0 {
                    material.isDoubleSided = true
                    material.blendMode = .alpha
                }
            } else {
                material.diffuse.contents = NSColor.gray
            }
            material.lightingModel = .blinn
            material.isDoubleSided = true
            geometry.materials = [material]

            let node = SCNNode(geometry: geometry)
            node.name = "\(mesh.entityId)"

            // IFC uses Z-up, SceneKit uses Y-up. Rotate to correct orientation.
            // This rotates -90 degrees around X axis to convert Z-up to Y-up
            node.eulerAngles.x = -CGFloat.pi / 2

            return node
        }

        private func fitCameraToBounds(scnView: SCNView, bounds: SceneBounds) {
            let centerX = (bounds.minX + bounds.maxX) / 2
            let centerY = (bounds.minY + bounds.maxY) / 2
            let centerZ = (bounds.minZ + bounds.maxZ) / 2

            let sizeX = bounds.maxX - bounds.minX
            let sizeY = bounds.maxY - bounds.minY
            let sizeZ = bounds.maxZ - bounds.minZ
            let maxSize = max(sizeX, max(sizeY, sizeZ))

            let distance = maxSize * 1.5

            if let cameraNode = scnView.pointOfView {
                SCNTransaction.begin()
                SCNTransaction.animationDuration = 0.5
                cameraNode.position = SCNVector3(
                    centerX + distance * 0.7,
                    centerY + distance * 0.5,
                    centerZ + distance * 0.7
                )
                cameraNode.look(at: SCNVector3(centerX, centerY, centerZ))
                SCNTransaction.commit()
            }
        }

        func zoomToEntity(scnView: SCNView, entityId: UInt64) {
            guard let node = meshNodes[entityId] else { return }

            // Get the bounding box of the node in world coordinates
            let (minBound, maxBound) = node.boundingBox
            let worldMin = node.convertPosition(minBound, to: nil)
            let worldMax = node.convertPosition(maxBound, to: nil)

            // Calculate center and size
            let centerX = (worldMin.x + worldMax.x) / 2
            let centerY = (worldMin.y + worldMax.y) / 2
            let centerZ = (worldMin.z + worldMax.z) / 2
            let center = SCNVector3(centerX, centerY, centerZ)

            let sizeX = Swift.abs(worldMax.x - worldMin.x)
            let sizeY = Swift.abs(worldMax.y - worldMin.y)
            let sizeZ = Swift.abs(worldMax.z - worldMin.z)
            let maxSize = Swift.max(sizeX, Swift.max(sizeY, sizeZ))

            // Calculate camera distance (ensure minimum distance for small objects)
            let distance = Swift.max(maxSize * 2.5, 5.0)

            if let cameraNode = scnView.pointOfView {
                SCNTransaction.begin()
                SCNTransaction.animationDuration = 0.5

                // Position camera at an angle to the object
                let camX = centerX + distance * 0.7
                let camY = centerY + distance * 0.5
                let camZ = centerZ + distance * 0.7
                cameraNode.position = SCNVector3(camX, camY, camZ)
                cameraNode.look(at: center)

                SCNTransaction.commit()
            }
        }

        @objc func handleTap(_ gesture: NSClickGestureRecognizer) {
            guard let scnView = sceneView else { return }
            let location = gesture.location(in: scnView)

            let hitResults = scnView.hitTest(location, options: [.searchMode: SCNHitTestSearchMode.closest.rawValue])

            if let hit = hitResults.first, let nodeName = hit.node.name, let entityId = UInt64(nodeName) {
                DispatchQueue.main.async {
                    self.parent.viewModel.select(entityId)
                }
            }
        }
    }
}
