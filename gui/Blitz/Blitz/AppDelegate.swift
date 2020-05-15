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
        workspace = Workspace();
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
        let resp = panel.runModal();
        if resp == NSApplication.ModalResponse.cancel {
            return;
        }
        let file = panel.url!.path;
        self.workspace.setFilename(filename: file);
    }
}

class Workspace : ObservableObject {
    @Published var filename: String? = nil
    @Published var preview: NSImage? = nil
    @Published var loaded: Bool = false

    var renderer: Renderer? = nil
    
    func setFilename(filename: String) {
        self.filename = filename;
        self.renderer = Renderer(fromFilename: filename);
        self.preview = self.renderer!.loadPreview();
        self.loaded = true;
    }
}
