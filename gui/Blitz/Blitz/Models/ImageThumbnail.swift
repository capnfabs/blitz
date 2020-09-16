//
//  ImageThumbnail.swift
//  Blitz
//
//  Created by Fabian Tamp on 16/5/20.
//  Copyright © 2020 Fabian Tamp. All rights reserved.
//

import Foundation
import AppKit

struct ImageThumbnail: Identifiable {
    let id = UUID()
    let path: URL
    let previewBytes: Data
    let renderer: Renderer
    
    init(path: URL) {
        self.path = path;
        self.renderer = Renderer(fromFilename: path.path)
        self.previewBytes = self.renderer.loadPreviewBytes();
    }
}

class AsyncImage : ObservableObject {
    @Published var image: NSImage?
    @Published var lastImage: NSImage?
    @Published var histogram: NSImage?
    
    var renderer: Renderer
    private var loading = false
    
    init(_ renderer: Renderer) {
        self.renderer = renderer
    }
    
    func loadWithSettings(settings: RenderSettings) {
        if !loading {
            print("Loading, with settings...")
            lastImage = image
            image = nil
            loading = true
            DispatchQueue.global().async {
                let (image, histo) = self.renderer.render(withSettings: settings)
                DispatchQueue.main.async {
                    self.image = image
                    self.histogram = histo
                    self.loading = false
                }
            }
        }
    }

    func cancel() {
        // Ignored
    }
    
}
