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
        return Renderer.toData(preview);
        
    }
    
    private static func toData(_ buffer: Buffer) -> Data {
        print("Referencing  \(buffer.len) bytes at \(buffer.data)")
        return Data(bytesNoCopy: buffer.data, count: Int(buffer.len), deallocator: .custom({(ptr, len) in
            print("Dropping \(len) bytes at \(ptr)")
            free_buffer(Buffer(data: ptr.assumingMemoryBound(to: UInt8.self), len: UInt(len)));
        }));
    }
    
    func render() -> Data {
        let result = raw_renderer_render_image(self.renderer);
        return Renderer.toData(result)
    }
    
    deinit {
        raw_renderer_free(self.renderer);
    }
}
