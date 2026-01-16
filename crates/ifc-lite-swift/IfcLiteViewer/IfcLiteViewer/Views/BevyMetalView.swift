import SwiftUI
import QuartzCore

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
class BevyViewController {
    private var metalView: MetalLayerView?
    #if os(iOS)
    private var displayLink: CADisplayLink?
    #endif
    private var bevyApp: OpaquePointer?
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

        // Create the Bevy app
        // Note: This requires the IfcLiteBevy framework to be linked
        // Uncomment when the framework is available:
        // let viewPtr = Unmanaged.passUnretained(view).toOpaque()
        // #if os(macOS)
        // let maxFps: Int32 = 60
        // #else
        // let maxFps = Int32(UIScreen.main.maximumFramesPerSecond)
        // #endif
        // bevyApp = create_bevy_app(viewPtr, maxFps, scaleFactor)

        isRunning = true
        startDisplayLink()
    }

    /// Stop the Bevy app and clean up
    func stop() {
        guard isRunning else { return }

        stopDisplayLink()

        if bevyApp != nil {
            // Uncomment when framework is linked:
            // release_bevy_app(bevyApp)
            bevyApp = nil
        }

        isRunning = false
        metalView = nil
    }

    /// Process a single frame
    @objc private func renderFrame() {
        guard isRunning, bevyApp != nil else { return }
        // Uncomment when framework is linked:
        // enter_frame(bevyApp)
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

    /// Load geometry from the view model's meshes
    func loadGeometry(meshesJson: String) -> Bool {
        guard bevyApp != nil else { return false }
        // return load_geometry(app, meshesJson)
        return true
    }

    /// Load entities from the view model
    func loadEntities(entitiesJson: String) -> Bool {
        guard bevyApp != nil else { return false }
        // return load_entities(app, entitiesJson)
        return true
    }

    // MARK: - Selection

    func select(entityId: UInt64) {
        guard bevyApp != nil else { return }
        // select_entity(app, entityId)
    }

    func clearSelection() {
        guard bevyApp != nil else { return }
        // clear_selection(app)
    }

    // MARK: - Visibility

    func hide(entityId: UInt64) {
        guard bevyApp != nil else { return }
        // hide_entity(app, entityId)
    }

    func show(entityId: UInt64) {
        guard bevyApp != nil else { return }
        // show_entity(app, entityId)
    }

    func showAll() {
        guard bevyApp != nil else { return }
        // show_all(app)
    }

    func isolate(entityIds: [UInt64]) {
        guard bevyApp != nil else { return }
        // entityIds.withUnsafeBufferPointer { buffer in
        //     isolate_entities(app, buffer.baseAddress, buffer.count)
        // }
    }

    // MARK: - Camera

    func cameraHome() {
        guard bevyApp != nil else { return }
        // camera_home(app)
    }

    func cameraFitAll() {
        guard bevyApp != nil else { return }
        // camera_fit_all(app)
    }

    func cameraFocus(entityId: UInt64) {
        guard bevyApp != nil else { return }
        // camera_focus_entity(app, entityId)
    }

    // MARK: - Touch/Mouse Input

    func touchBegan(at point: CGPoint) {
        guard bevyApp != nil else { return }
        // touch_started(app, Float(point.x), Float(point.y))
    }

    func touchMoved(to point: CGPoint) {
        guard bevyApp != nil else { return }
        // touch_moved(app, Float(point.x), Float(point.y))
    }

    func touchEnded(at point: CGPoint) {
        guard bevyApp != nil else { return }
        // touch_ended(app, Float(point.x), Float(point.y))
    }

    func touchCancelled(at point: CGPoint) {
        guard bevyApp != nil else { return }
        // touch_cancelled(app, Float(point.x), Float(point.y))
    }

    // MARK: - Theme

    func setTheme(dark: Bool) {
        guard bevyApp != nil else { return }
        // set_theme(app, dark)
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
