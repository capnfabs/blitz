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
    var id = UUID()
    var path: URL
    var previewBytes: Data
    var renderer: Renderer
    
    init(path: URL) {
        self.path = path;
        self.renderer = Renderer(fromFilename: path.path)
        self.previewBytes = self.renderer.loadPreviewBytes();
    }
}

class AsyncImage : ObservableObject {
    @Published var image: Data?
    var renderer: Renderer
    private var loading = false
    
    init(_ renderer: Renderer) {
        self.renderer = renderer
    }
    
    
    func load() {
        print("Loading...")
        if image == nil && !loading {
            loading = true
            DispatchQueue.global().async {
                let bytes = self.renderer.render()
                DispatchQueue.main.async {
                    self.image = bytes
                    self.loading = false
                }
            }
        }
    }
    
    func loadWithSettings(settings: RenderSettings) {
        if !loading {
            print("Loading, with settings...")
            image = nil
            loading = true
            DispatchQueue.global().async {
                let bytes = self.renderer.render(withSettings: settings)
                DispatchQueue.main.async {
                    self.image = bytes
                    self.loading = false
                }
            }
        }
    }

    func cancel() {
        // Ignored
    }
    
}
