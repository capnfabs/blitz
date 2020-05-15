//
//  RendererWrapper.swift
//  Blitz
//
//  Created by Fabian Tamp on 15/5/20.
//  Copyright © 2020 Fabian Tamp. All rights reserved.
//

import Foundation
import AppKit

class Renderer {
    var renderer: OpaquePointer
    
    init(fromFilename filename: String) {
        self.renderer = raw_renderer_new(filename)!;
    }
    
    func loadPreview() -> NSImage {
        let preview = raw_renderer_get_preview(self.renderer);
        let data = Data(bytesNoCopy: preview.data, count: Int(preview.len), deallocator: Data.Deallocator.custom({(ptr, len) in free_buffer(preview)}));
        return NSImage(data: data)!;
    }
    
    deinit {
        raw_renderer_free(self.renderer);
    }
}
