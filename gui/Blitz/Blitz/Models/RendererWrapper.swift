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

extension RawImage {
    func toNSImage() -> NSImage {
        let data = self.data.toData()
        // TODO: this will all break if we introduce another format; but I don't know swift well enough to mess around with preventing this yet.
        let samplesPerPixel = self.pixel_format == Rgb ? 3 : 4
        let hasAlpha = self.pixel_format == Rgba
        let bitsPerSample = 8
        let bitsPerPixel = bitsPerSample * samplesPerPixel
        
        let rep = data.withUnsafeBytes { (bytes) -> NSBitmapImageRep in
            let imgptr = UnsafeMutablePointer(mutating: bytes.bindMemory(to: UInt8.self).baseAddress)
            let wut = [imgptr]
            return wut.withUnsafeBufferPointer { (arrayPtr) -> NSBitmapImageRep in
                let dataPlanes = UnsafeMutablePointer(mutating: arrayPtr.baseAddress!)
                return NSBitmapImageRep(bitmapDataPlanes: dataPlanes, pixelsWide: Int(self.width), pixelsHigh: Int(self.height), bitsPerSample: bitsPerSample, samplesPerPixel: samplesPerPixel, hasAlpha: hasAlpha, isPlanar: false, colorSpaceName: .calibratedRGB, bytesPerRow: Int(self.width)*samplesPerPixel, bitsPerPixel: bitsPerPixel)!
            }
        }
        let img = NSImage()
        img.addRepresentation(rep)
        return img
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
    
    func render() -> NSImage {
        let result = raw_renderer_render_image(self.renderer);
        return result.toNSImage()
    }
    
    func render(withSettings: RenderSettings) -> (NSImage, NSImage) {
        let result = raw_renderer_render_with_settings(self.renderer, withSettings);
        return (result.img.toNSImage(), result.histogram.toNSImage())
    }
    
    deinit {
        raw_renderer_free(self.renderer);
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
