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
    
    init(path: URL) {
        self.path = path;
        self.previewBytes = Renderer(fromFilename: path.path).loadPreviewBytes();
    }
}
