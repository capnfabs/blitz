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

    @State private var currentImage: AsyncImage?
    @State private var currentImageUrl: URL?
    @EnvironmentObject var workspace: Workspace;
    
    var body: some View {
        Group {
            if !workspace.loaded {
                Text("Hi there! Please choose a directory.")
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                HSplitView{
                    LibraryView(onSelect: {thumb in
                        print("Selected: \(thumb.path)")
                        self.currentImage = AsyncImage(thumb.renderer)
                        self.currentImageUrl = thumb.path
                    }, minimalMode: detailExpanded())
                    if (detailExpanded()) {
                        DetailView(filename: self.currentImageUrl!.lastPathComponent, image: currentImage!)
                    }
                }
            }
        }
    }

    func detailExpanded() -> Bool {
        return currentImage != nil && currentImageUrl != nil
    }
}

struct LibraryView: View {
    @EnvironmentObject var workspace: Workspace;
    let onSelect: (ImageThumbnail) -> Void
    let minimalMode: Bool
    
    var body: some View {
        VStack {
            Text("Directory: \(self.workspace.directory!.path)")
            QGrid(self.workspace.previews, columns: self.minimalMode ? 1 : 4, isScrollable: true, showScrollIndicators: true) { preview in
                Button(action: { self.onSelect(preview) }){
                    ImageTile(image: preview)
                }.buttonStyle(PlainButtonStyle())
            }
        }.frame(minWidth: 200, maxWidth: self.minimalMode ? 400 : .infinity, minHeight: 400, maxHeight: .infinity)
    }
}

class SlideyCoordinator: NSObject {

    @Binding var value: Double

    init(value: Binding<Double>) {
        _value = value
    }
    
    @objc func valueChanged(_ sender: NSSlider) {
        self.value = sender.doubleValue
    }
}

struct SlideyBoi: NSViewRepresentable {
    @Binding var value: Double
    
    var vertical: Bool = false
    var min: Double = 0.0
    var max: Double = 1.0
    
    func makeCoordinator() -> SlideyCoordinator {
        return SlideyCoordinator(value: $value)
    }
    
    func makeNSView(context: Context) -> NSSlider {
        let slider = NSSlider(value: self.value, minValue: self.min, maxValue: self.max, target: context.coordinator, action: #selector(SlideyCoordinator.valueChanged))
        slider.isVertical = self.vertical
        return slider
    }

    func updateNSView(_ nsView: NSSlider, context: Context) {
        nsView.doubleValue = value
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

struct RenderControlsView_Previews: PreviewProvider {
    static var previews: some View {
        return RenderControlsView(onUpdateClicked: {_ in })
    }
}

struct InlineButtonStyle: ButtonStyle {
 
    func makeBody(configuration: Self.Configuration) -> some View {
        configuration.label
        .padding(5)
            .foregroundColor(.white)
            .background(configuration.isPressed ? Color.accentColor : Color.gray)
        .cornerRadius(5)
    }
}
