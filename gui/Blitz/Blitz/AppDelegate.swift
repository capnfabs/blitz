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
        print("Hi!!");
        let panel = NSOpenPanel();
        panel.canChooseDirectories = true;
        panel.canChooseFiles = false;
        let resp = panel.runModal();
        if resp == NSApplication.ModalResponse.cancel {
            return;
        }
        let file = panel.url!.path;
        self.workspace.setDirectory(path: file);
    }
}

class Workspace : ObservableObject, Codable {
    @Published var directory: String? = nil
    @Published var previews: [ImageThumbnail] = []
    @Published var loaded: Bool = false
    
    enum CodingKeys: CodingKey {
        case directory
    }
    
    func setDirectory(path: String) {
        self.directory = path;
    
        // TODO: this is predicated on Workspace being a singleton.
        saveState();
        
        loadPreviews();
        
        self.loaded = true;
    }
    
    class func getPath() -> URL {
        var path = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!;
        path.appendPathComponent("workspace.json");
        return path;
    }
    
    func saveState() {
        let encoder = JSONEncoder();
        do {
            let data = try encoder.encode(self);
            try data.write(to: Workspace.getPath())
        } catch {
            // TODO
        }
    }
    
    func loadPreviews() {
        // Loads all previews for a directory. Is kinda slow, probably because it's happening on the main thread.
        previews.removeAll();
        let fm = FileManager.default
        
        let enumerator = fm.enumerator(at: URL(fileURLWithPath: self.directory!), includingPropertiesForKeys: nil)!
        for file in enumerator.filter({($0 as! URL).path.lowercased().hasSuffix(".raf")}) {
            previews.append(ImageThumbnail(path: file as! URL))
        }
    }
    
    class func fromStorage() -> Workspace? {
        let decoder = JSONDecoder();
        do {
            return try decoder.decode(Workspace.self, from: Data(contentsOf: getPath()));
        } catch {
            // TODO
        }
        return nil;
    }
    
    init() {
    }
    
    required init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        directory = try container.decode(String.self, forKey: .directory)
        if directory != nil {
            loadPreviews();
            loaded = true;
        }
    }
    
    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(directory, forKey: .directory)
    }
}
