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
                        self.currentImage!.load()
                    }, minimalMode: detailExpanded())
                    if (detailExpanded()) {
                        DetailView(image: currentImage!)
                    }
                }
            }
        }
    }

    func detailExpanded() -> Bool {
        return currentImage != nil
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

struct DetailView: View {
    @ObservedObject var image: AsyncImage;
    // TODO: Something to make this size consistently regardless of whether there's already a view or not.
    var body: some View {
        return VStack {
            Group {
                if image.image != nil {
                    Image(nsImage: image.image!.toNSImage())
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                } else {
                    Text("Loading; stand by...")
                    .padding(20)
                }
            }
            RenderControlsView()
        }
        .frame(minWidth: 400, maxWidth: .infinity, minHeight: 400, maxHeight: .infinity)
    }
}

struct RenderControlsView: View {
    @State var curve0: Double = 0
    @State var curve1: Double = 0
    @State var curve2: Double = 0
    @State var curve3: Double = 0
    @State var curve4: Double = 0
    
    var body: some View {
        VStack {
            Text("Slidey bois! \(curve0)")
            HStack {
                SlideyBoi(value: $curve0)
                SlideyBoi(value: $curve1)
                SlideyBoi(value: $curve2)
                SlideyBoi(value: $curve3)
                SlideyBoi(value: $curve4)
            }
        }
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
    
    func makeCoordinator() -> SlideyCoordinator {
        return SlideyCoordinator(value: $value)
    }
    
    func makeNSView(context: Context) -> NSSlider {
        let slider = NSSlider(value: self.value, minValue: -5, maxValue: 5, target: context.coordinator, action: #selector(SlideyCoordinator.valueChanged))
        slider.isVertical = true
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



struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView().environmentObject(Workspace());
    }
}
