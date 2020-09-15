//
//  AppDelegate.swift
//  Blitz
//
//  Created by Fabian Tamp on 15/5/20.
//  Copyright © 2020 Fabian Tamp. All rights reserved.
//

import Cocoa
import SwiftUI

@NSApplicationMain
class AppDelegate: NSObject, NSApplicationDelegate {

    var window: NSWindow!
    var workspace: Workspace!
    var renderCache: RenderCache!
    
    @State private var currentImageFilename: String?;


    func applicationDidFinishLaunching(_ aNotification: Notification) {
        // Create the SwiftUI view that provides the window contents.
        workspace = Workspace.fromStorage() ?? Workspace();
        let contentView = ContentView().environmentObject(workspace);

        // Create the window and set the content view. 
        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 480, height: 300),
            styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
            backing: .buffered, defer: false)
        window.center()
        window.setFrameAutosaveName("Main Window")
        window.contentView = NSHostingView(rootView: contentView)
        window.makeKeyAndOrderFront(nil)
    }

    func applicationWillTerminate(_ aNotification: Notification) {
        // Insert code here to tear down your application
    }
    
    @IBAction func openDocument(_ sender: Any) {
        let panel = NSOpenPanel();
        panel.canChooseDirectories = true;
        panel.canChooseFiles = false;
        let resp = panel.runModal();
        if resp == NSApplication.ModalResponse.cancel {
            return;
        }
        let file = panel.url!;
        self.workspace.setDirectory(path: file);
    }
    
    @IBAction func chooseOutputDirectory(_ sender: Any) {
        let panel = NSOpenPanel();
        panel.canChooseDirectories = true;
        panel.canChooseFiles = false;
        let resp = panel.runModal();
        if resp == NSApplication.ModalResponse.cancel {
            return;
        }
        let file = panel.url!;
        self.workspace.setOutputDirectory(path: file);
    }
    
    func saveRender(label: String, data: Data) {
        print("Saving render", label)
        if self.workspace.outputDirectory == nil {
            chooseOutputDirectory(self)
            if self.workspace.outputDirectory == nil {
                // We still don't have an export dir after prompting the user, give up
                // TODO: this is so horribly side-effecty, refactor it.
                return
            }
        }
        
        // export filename:
        let df = DateFormatter()
        df.dateFormat = "yyyy-MM-dd-HH.mm.ss"
        let date = df.string(from: Date())
        
        let outputUrl = self.workspace.outputDirectory!.appendingPathComponent("blitzgui-\(date).jpg")
        data.toNSImage().saveJpegToUrl(url: outputUrl)
    }
}

class Workspace : ObservableObject, Codable {
    @Published var directory: URL? = nil
    var outputDirectory: URL? = nil
    @Published var previews: [ImageThumbnail] = []
    @Published var loaded: Bool = false
    
    enum CodingKeys: CodingKey {
        case directory
        case outputDirectory
    }
    
    func setDirectory(path: URL) {
        self.directory = path;
    
        // TODO: this is predicated on Workspace being a singleton.
        saveState();
        
        loadPreviews();
        
        self.loaded = true;
    }
    
    func setOutputDirectory(path: URL) {
        print("Setting output directory to", path)
        self.outputDirectory = path
        saveState()
    }
    
    class func getPath() -> URL {
        var path = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!;
        path.appendPathComponent("workspace.json");
        return path;
    }
    
    private func saveState() {
        let encoder = JSONEncoder();
        do {
            let data = try encoder.encode(self);
            try data.write(to: Workspace.getPath())
        } catch {
            // TODO
        }
    }
    
    private func loadPreviews() {
        // Loads all previews for a directory. Is kinda slow, probably because it's happening on the main thread.
        previews.removeAll();
        let fm = FileManager.default
        
        let enumerator = fm.enumerator(at: self.directory!, includingPropertiesForKeys: nil)!
        for file in enumerator.filter({($0 as! URL).path.lowercased().hasSuffix(".raf")}) {
            previews.append(ImageThumbnail(path: file as! URL))
        }
    }
    
    class func fromStorage() -> Workspace? {
        let decoder = JSONDecoder();
        do {
            let path = getPath()
            print("Attempting load from \(path)")
            return try decoder.decode(Workspace.self, from: Data(contentsOf: path));
        } catch {
            print("Load failed.")
            // TODO
        }
        return nil;
    }
    
    init() {
    }
    
    required init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        directory = try container.decode(URL.self, forKey: .directory)
        outputDirectory = try container.decode(URL.self, forKey: .outputDirectory)
        if directory != nil {
            loadPreviews();
            loaded = true;
        }
    }
    
    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(directory, forKey: .directory)
        try container.encode(outputDirectory, forKey: .outputDirectory)
    }
}
