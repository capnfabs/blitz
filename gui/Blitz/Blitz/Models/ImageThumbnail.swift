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
    @Published var image: Data?
    @Published var lastImage: Data?
    @Published var histogram: Data?
    
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
                let (bytes, histo_bytes) = self.renderer.render(withSettings: settings)
                DispatchQueue.main.async {
                    self.image = bytes
                    self.histogram = histo_bytes
                    self.loading = false
                }
            }
        }
    }

    func cancel() {
        // Ignored
    }
    
}
