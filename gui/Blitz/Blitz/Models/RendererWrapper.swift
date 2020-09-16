//
//  RendererWrapper.swift
//  Blitz
//
//  Created by Fabian Tamp on 15/5/20.
//  Copyright © 2020 Fabian Tamp. All rights reserved.
//

import Foundation
import AppKit


extension Buffer {
    func toData() -> Data {
        if self.data == nil {
            return Data()
        }
        print("Referencing  \(self.len) bytes at \(self.data!)")
        return Data(bytesNoCopy: self.data, count: Int(self.len), deallocator: .custom({(ptr, len) in
            print("Dropping \(len) bytes at \(ptr)")
            free_buffer(Buffer(data: ptr.assumingMemoryBound(to: UInt8.self), len: UInt(len)));
        }));
    }
}

class Renderer {
    var renderer: OpaquePointer
    
    init(fromFilename filename: String) {
        self.renderer = raw_renderer_new(filename)!;
    }
    
    func loadPreviewBytes() -> Data {
        let preview = raw_renderer_get_preview(self.renderer);
        return preview.toData();
        
    }
    
    func render() -> Data {
        let result = raw_renderer_render_image(self.renderer);
        return result.toData()
    }
    
    func render(withSettings: RenderSettings) -> (Data, Data) {
        let result = raw_renderer_render_with_settings(self.renderer, withSettings);
        return (result.img.toData(), result.histogram.toData())
    }
    
    deinit {
        raw_renderer_free(self.renderer);
    }
}

extension Data {
    func toNSImage() -> NSImage {
        let rep = self.withUnsafeBytes { (bytes) -> NSBitmapImageRep in
            let imgptr = UnsafeMutablePointer(mutating: bytes.bindMemory(to: UInt8.self).baseAddress)
            let wut = [imgptr]
            return wut.withUnsafeBufferPointer { (arrayPtr) -> NSBitmapImageRep in
                let dataPlanes = UnsafeMutablePointer(mutating: arrayPtr.baseAddress!)
                return NSBitmapImageRep(bitmapDataPlanes: dataPlanes, pixelsWide: 6000, pixelsHigh: 4000, bitsPerSample: 8, samplesPerPixel: 3, hasAlpha: false, isPlanar: false, colorSpaceName: .calibratedRGB, bytesPerRow: 6000*3, bitsPerPixel: 24)!
            }
        }
        let img = NSImage()
        img.addRepresentation(rep)
        return img
    }
}

extension NSImage {
    func saveJpegToUrl(url: URL) {
        let data = self.tiffRepresentation
        let rep = NSBitmapImageRep(data: data!)
        let imgData = rep!.representation(using: .jpeg, properties: [.compressionFactor: 0.95])
        do {
            try imgData!.write(to: url)
        } catch {
            print("TODO")
        }
    }
}
