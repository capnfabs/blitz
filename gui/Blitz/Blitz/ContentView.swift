//
//  ContentView.swift
//  Blitz
//
//  Created by Fabian Tamp on 15/5/20.
//  Copyright © 2020 Fabian Tamp. All rights reserved.
//

import SwiftUI

struct ContentView: View {
    
    @Binding var currentImageFilename: String?;
    
    var body: some View {
        VStack {
            if currentImageFilename != nil {
                Image(nsImage: loadImage(filename: self.currentImageFilename!))
            } else {
                Text("Hi there! Please open a file.")
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
    }
    
    func loadImage(filename: String) -> NSImage {
        Renderer(fromFilename: filename).loadPreview()
    }
}


struct ContentView_Previews: PreviewProvider {
    @State static var currentImageFilename: String? = nil;
    static var previews: some View {
        ContentView(currentImageFilename: self.$currentImageFilename);
    }
}
