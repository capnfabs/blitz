//
//  RendererWrapper.swift
//  Blitz
//
//  Created by Fabian Tamp on 15/5/20.
//  Copyright © 2020 Fabian Tamp. All rights reserved.
//

import Foundation

class Renderer {
    var renderer: OpaquePointer
    
    init(fromFilename filename: String) {
        self.renderer = raw_renderer_new(filename)!;
    }
    
    func loadPreviewBytes() -> Data {
        let preview = raw_renderer_get_preview(self.renderer);
        let data = Data(bytesNoCopy: preview.data, count: Int(preview.len), deallocator: .custom({(ptr, len) in
            print("Dealloc!");
            free_buffer(Buffer(data: ptr.assumingMemoryBound(to: UInt8.self), len: UInt(len)));
        }));
        print("i");
        return data;
    }
    
    deinit {
        raw_renderer_free(self.renderer);
    }
}
