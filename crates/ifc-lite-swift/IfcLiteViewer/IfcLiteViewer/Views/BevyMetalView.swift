import SwiftUI
import QuartzCore
import IfcLite

#if os(macOS)
import AppKit
typealias PlatformView = NSView
typealias PlatformViewRepresentable = NSViewRepresentable
#else
import UIKit
typealias PlatformView = UIView
typealias PlatformViewRepresentable = UIViewRepresentable
#endif

// MARK: - Metal View

/// A view backed by CAMetalLayer for Bevy rendering
class MetalLayerView: PlatformView {
    #if os(macOS)
    override func makeBackingLayer() -> CALayer {
        let metalLayer = CAMetalLayer()
        metalLayer.contentsScale = NSScreen.main?.backingScaleFactor ?? 2.0
        return metalLayer
    }

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        configureLayer()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        wantsLayer = true
        configureLayer()
    }

    private func configureLayer() {
        guard let layer = self.layer as? CAMetalLayer else { return }
        layer.pixelFormat = .bgra8Unorm
        layer.framebufferOnly = true
        layer.presentsWithTransaction = false
        layer.backgroundColor = CGColor(red: 0.1, green: 0.1, blue: 0.1, alpha: 1.0)
    }

    var scaleFactor: CGFloat {
        window?.backingScaleFactor ?? NSScreen.main?.backingScaleFactor ?? 2.0
    }
    #else
    override class var layerClass: AnyClass {
        return CAMetalLayer.self
    }

    override init(frame: CGRect) {
        super.init(frame: frame)
        configureLayer()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        configureLayer()
    }

    private func configureLayer() {
        guard let layer = self.layer as? CAMetalLayer else { return }
        layer.pixelFormat = .bgra8Unorm
        layer.framebufferOnly = true
        layer.presentsWithTransaction = false
        layer.backgroundColor = CGColor(red: 0.1, green: 0.1, blue: 0.1, alpha: 1.0)
        contentScaleFactor = UIScreen.main.nativeScale
    }

    var scaleFactor: CGFloat {
        UIScreen.main.nativeScale
    }
    #endif
}

// MARK: - Bevy View Controller

/// Controller that manages the Bevy app lifecycle and frame updates
/// Now uses UniFFI-generated BevyViewer from IfcLite
class BevyViewController {
    private var metalView: MetalLayerView?
    #if os(iOS)
    private var displayLink: CADisplayLink?
    #endif
    private var bevyViewer: BevyViewer?
    private var isRunning = false

    // Callbacks for events from Bevy
    var onEntitySelected: ((UInt64) -> Void)?
    var onEntityHovered: ((UInt64?) -> Void)?

    init() {}

    deinit {
        stop()
    }

    /// Attach to a MetalLayerView and start the Bevy app
    func start(with view: MetalLayerView) {
        guard !isRunning else { return }

        metalView = view
        let scaleFactor = Float(view.scaleFactor)

        // Get view pointer as UInt64 for UniFFI
        let viewPtr = UInt64(UInt(bitPattern: Unmanaged.passUnretained(view).toOpaque()))

        // Create the Bevy viewer via UniFFI
        bevyViewer = BevyViewer(viewPtr: viewPtr, scaleFactor: scaleFactor)

        isRunning = true
        startDisplayLink()
    }

    /// Stop the Bevy app and clean up
    func stop() {
        guard isRunning else { return }

        stopDisplayLink()

        if let viewer = bevyViewer {
            viewer.stop()
            bevyViewer = nil
        }

        isRunning = false
        metalView = nil
    }

    /// Process a single frame
    @objc private func renderFrame() {
        guard isRunning, let viewer = bevyViewer else { return }
        viewer.enterFrame()
    }

    // MARK: - Display Link

    #if os(macOS)
    private var cvDisplayLink: CVDisplayLink?

    private func startDisplayLink() {
        var displayLink: CVDisplayLink?
        CVDisplayLinkCreateWithActiveCGDisplays(&displayLink)

        guard let link = displayLink else { return }

        let callback: CVDisplayLinkOutputCallback = { _, _, _, _, _, userInfo -> CVReturn in
            let controller = Unmanaged<BevyViewController>.fromOpaque(userInfo!).takeUnretainedValue()
            DispatchQueue.main.async {
                controller.renderFrame()
            }
            return kCVReturnSuccess
        }

        let userInfo = Unmanaged.passUnretained(self).toOpaque()
        CVDisplayLinkSetOutputCallback(link, callback, userInfo)
        CVDisplayLinkStart(link)
        cvDisplayLink = link
    }

    private func stopDisplayLink() {
        if let link = cvDisplayLink {
            CVDisplayLinkStop(link)
            cvDisplayLink = nil
        }
    }
    #else
    private func startDisplayLink() {
        displayLink = CADisplayLink(target: self, selector: #selector(renderFrame))
        displayLink?.preferredFramesPerSecond = 60
        displayLink?.add(to: .main, forMode: .common)
    }

    private func stopDisplayLink() {
        displayLink?.invalidate()
        displayLink = nil
    }
    #endif

    // MARK: - Data Loading

    /// Load geometry from JSON
    func loadGeometry(meshesJson: String) -> Bool {
        guard let viewer = bevyViewer else { return false }
        return viewer.loadGeometry(meshesJson: meshesJson)
    }

    /// Load entities from JSON
    func loadEntities(entitiesJson: String) -> Bool {
        guard let viewer = bevyViewer else { return false }
        return viewer.loadEntities(entitiesJson: entitiesJson)
    }

    // MARK: - Selection

    func select(entityId: UInt64) {
        bevyViewer?.selectEntity(entityId: entityId)
    }

    func clearSelection() {
        bevyViewer?.clearSelection()
    }

    // MARK: - Visibility

    func hide(entityId: UInt64) {
        bevyViewer?.hideEntity(entityId: entityId)
    }

    func show(entityId: UInt64) {
        bevyViewer?.showEntity(entityId: entityId)
    }

    func showAll() {
        bevyViewer?.showAll()
    }

    // MARK: - Camera

    func cameraHome() {
        bevyViewer?.cameraHome()
    }

    func cameraFitAll() {
        bevyViewer?.cameraFitAll()
    }

    func cameraFocus(entityId: UInt64) {
        bevyViewer?.cameraFocusEntity(entityId: entityId)
    }

    // MARK: - Touch/Mouse Input

    func touchBegan(at point: CGPoint) {
        bevyViewer?.touchStarted(x: Float(point.x), y: Float(point.y))
    }

    func touchMoved(to point: CGPoint) {
        bevyViewer?.touchMoved(x: Float(point.x), y: Float(point.y))
    }

    func touchEnded(at point: CGPoint) {
        bevyViewer?.touchEnded(x: Float(point.x), y: Float(point.y))
    }

    func touchCancelled(at point: CGPoint) {
        bevyViewer?.touchCancelled(x: Float(point.x), y: Float(point.y))
    }

    // MARK: - Theme

    func setTheme(dark: Bool) {
        bevyViewer?.setTheme(dark: dark)
    }
}

// MARK: - SwiftUI Wrapper

#if os(macOS)
/// SwiftUI wrapper for the Bevy Metal view on macOS
struct BevyMetalView: NSViewRepresentable {
    @EnvironmentObject var viewModel: ViewerViewModel
    let controller: BevyViewController

    func makeNSView(context: Context) -> MetalLayerView {
        let view = MetalLayerView(frame: .zero)
        controller.start(with: view)

        // Set up event handlers
        controller.onEntitySelected = { [weak viewModel] entityId in
            viewModel?.select(entityId)
        }

        return view
    }

    func updateNSView(_ nsView: MetalLayerView, context: Context) {
        // Sync state from SwiftUI to Bevy
        controller.setTheme(dark: viewModel.isDarkMode)

        // Update selection if changed
        if let selectedId = viewModel.selectedIds.first {
            controller.select(entityId: selectedId)
        }
    }

    static func dismantleNSView(_ nsView: MetalLayerView, coordinator: ()) {
        // Cleanup is handled by the controller's deinit
    }
}
#else
/// SwiftUI wrapper for the Bevy Metal view on iOS
struct BevyMetalView: UIViewRepresentable {
    @EnvironmentObject var viewModel: ViewerViewModel
    let controller: BevyViewController

    func makeUIView(context: Context) -> MetalLayerView {
        let view = MetalLayerView(frame: .zero)
        view.isMultipleTouchEnabled = true
        controller.start(with: view)

        // Set up event handlers
        controller.onEntitySelected = { [weak viewModel] entityId in
            viewModel?.select(entityId)
        }

        // Add gesture recognizers
        let panGesture = UIPanGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.handlePan(_:)))
        view.addGestureRecognizer(panGesture)

        let pinchGesture = UIPinchGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.handlePinch(_:)))
        view.addGestureRecognizer(pinchGesture)

        let tapGesture = UITapGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.handleTap(_:)))
        view.addGestureRecognizer(tapGesture)

        return view
    }

    func updateUIView(_ uiView: MetalLayerView, context: Context) {
        controller.setTheme(dark: viewModel.isDarkMode)
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(controller: controller)
    }

    class Coordinator: NSObject {
        let controller: BevyViewController

        init(controller: BevyViewController) {
            self.controller = controller
        }

        @objc func handlePan(_ gesture: UIPanGestureRecognizer) {
            let location = gesture.location(in: gesture.view)
            switch gesture.state {
            case .began:
                controller.touchBegan(at: location)
            case .changed:
                controller.touchMoved(to: location)
            case .ended:
                controller.touchEnded(at: location)
            case .cancelled:
                controller.touchCancelled(at: location)
            default:
                break
            }
        }

        @objc func handlePinch(_ gesture: UIPinchGestureRecognizer) {
            // Pinch-to-zoom would be handled here
            // For now, we'll pass touch events
        }

        @objc func handleTap(_ gesture: UITapGestureRecognizer) {
            let location = gesture.location(in: gesture.view)
            controller.touchBegan(at: location)
            controller.touchEnded(at: location)
        }
    }
}
#endif

// MARK: - Preview

#Preview {
    BevyMetalView(controller: BevyViewController())
        .environmentObject(ViewerViewModel())
        .frame(width: 800, height: 600)
}
