//
//  ContentView.swift
//  Blitz
//
//  Created by Fabian Tamp on 15/5/20.
//  Copyright © 2020 Fabian Tamp. All rights reserved.
//

import SwiftUI
import QGrid

struct ContentView: View {
    
    @EnvironmentObject var workspace: Workspace;
    var renderCache: RenderCache;
    
    var body: some View {
        NavigationView {
            VStack {
                if !workspace.loaded {
                    Text("Hi there! Please choose a directory.")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else {
                    Text("Opened directory \(workspace.directory!)")
                    QGrid(workspace.previews, columns: 4, isScrollable: true, showScrollIndicators: true) { preview in
                        NavigationLink(destination: self.destinationForItem(preview)) {
                            ImageTile(image: preview).frame(minWidth: 200, minHeight: 200)
                        }.buttonStyle(PlainButtonStyle())
                    }.frame(minWidth: 0, maxWidth: .infinity, minHeight: 400, maxHeight: .infinity)
                }
            }
        }
    }
    
    func destinationForItem(_ item: ImageThumbnail) -> some View
    {
        ImageDetail(image: item.rendered)
    }
}

struct ImageDetail: View {
    @ObservedObject var image: AsyncImage;
    var body: some View {
        Group {
            if image.image != nil {
                Image(nsImage: render(image.image!))
                    .resizable()
            } else {
                Text("LOADING, STANDBY")
            }
        }.onAppear {
            self.image.load()
        }.onDisappear() {
            self.image.cancel()
        }
    }
    
    func render(_ image: Data) -> NSImage {
        let rep = image.withUnsafeBytes { (bytes) -> NSBitmapImageRep in
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

struct ImageTile: View {
    var image: ImageThumbnail;
    
    var body: some View {
        VStack {
            Text(image.path.lastPathComponent)
            
            Image(nsImage: NSImage(data: image.previewBytes)!)
                .resizable()
                .scaledToFit()
        }
        .padding(10)
        .clipShape(RoundedRectangle(cornerRadius: 2))
        .overlay(
            RoundedRectangle(cornerRadius: 4)
                .stroke(Color.gray, lineWidth: 2)
        )
        .padding(5)
    }
}

struct ImageTile_Previews: PreviewProvider {
    static var previews: some View {
        
        let filePath = "/Users/fabian/Downloads/camera/raw/DSCF2406.raf";
        
        let thumb = ImageThumbnail(path: URL(fileURLWithPath: filePath));
        return ImageTile(image: thumb);
    }
}



struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView(renderCache: RenderCache()).environmentObject(Workspace());
    }
}
