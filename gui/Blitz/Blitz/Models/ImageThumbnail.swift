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
    
    var rendered: AsyncImage
    
    init(path: URL) {
        self.path = path;
        self.renderer = Renderer(fromFilename: path.path)
        self.previewBytes = self.renderer.loadPreviewBytes();
        self.rendered = AsyncImage(self.renderer)
    }
}

class AsyncImage : ObservableObject {
    @Published var image: Data?
    var renderer: Renderer
    
    init(_ renderer: Renderer) {
        self.renderer = renderer
    }
    
    
    func load() {
        DispatchQueue.global().async {
            let bytes = self.renderer.render()
            DispatchQueue.main.async {
                self.image = bytes
            }
        }
    }

    func cancel() {
        // Not implemented
    }
    
}
