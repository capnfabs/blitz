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

// Extremely hot garbage: use this to hold a ref to a Data object within an NSImage.
class BridgeRawBitmapImageRep : NSBitmapImageRep {
    
    let data: Data
    let bufferWhatsits: [UnsafeMutablePointer<UInt8>?]
    
    init?(data: Data, pixelFormat: ImageFormat, width: Int, height: Int) {
        // Hold a ref to the data so it doesn't magically break sometime
        self.data = data
        
        // TODO: this will all break if we introduce another format; but I don't know swift well enough to mess around with preventing this yet.
        let samplesPerPixel = pixelFormat == Rgb ? 3 : 4
        let hasAlpha = pixelFormat == Rgba
        let bitsPerSample = 8
        let bitsPerPixel = bitsPerSample * samplesPerPixel
        
        
        self.bufferWhatsits = data.withUnsafeBytes { (bytes) -> [UnsafeMutablePointer<UInt8>?] in
            let imgptr = UnsafeMutablePointer(mutating: bytes.bindMemory(to: UInt8.self).baseAddress)
            return [imgptr]
        }
        
        let dataPlanes = self.bufferWhatsits.withUnsafeBufferPointer { (arrayPtr) -> UnsafeMutablePointer<UnsafeMutablePointer<UInt8>?>? in
            return UnsafeMutablePointer(mutating: arrayPtr.baseAddress!)
        }
        super.init(bitmapDataPlanes: dataPlanes, pixelsWide: width, pixelsHigh: height, bitsPerSample: bitsPerSample, samplesPerPixel: samplesPerPixel, hasAlpha: hasAlpha, isPlanar: false, colorSpaceName: .calibratedRGB, bitmapFormat: NSBitmapImageRep.Format(), bytesPerRow: Int(width)*samplesPerPixel, bitsPerPixel: bitsPerPixel)
    }
    
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
    
}

extension RawImage {
    func toNSImage() -> NSImage {
        let data = self.data.toData()
        let img = NSImage()
        let rep = BridgeRawBitmapImageRep(data: data, pixelFormat: self.pixel_format, width: Int(self.width), height: Int(self.height))!
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
