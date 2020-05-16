//
//  ImageTile.swift
//  Blitz
//
//  Created by Fabian Tamp on 16/5/20.
//  Copyright © 2020 Fabian Tamp. All rights reserved.
//

import SwiftUI

struct ImageTile: View {
    var image: ImageThumbnail;
    
    var body: some View {
        VStack(alignment: .leading) {
            Text("LOL \(image.path.lastPathComponent)")
            
            Image(nsImage: NSImage(data: image.previewBytes)!)
                .resizable().frame(width: 200, height: 200)
            Text("lot of bytes: \(image.previewBytes.count)")
            
            Text("Hi there")
        }
    }
}

struct ImageTile_Previews: PreviewProvider {
    static var previews: some View {
        
        let filePath = "/Users/fabian/Downloads/camera/raw/DSCF2406.raf";
        
        let thumb = ImageThumbnail(path: URL(fileURLWithPath: filePath));
        return ImageTile(image: thumb);
    }
}
