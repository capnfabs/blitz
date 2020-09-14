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
            RenderControlsView(onUpdateClicked: {
                print("lol!!", $0);
                self.image.loadWithSettings(settings: RenderSettings(tone_curve: $0))
            })
        }
        .frame(minWidth: 400, maxWidth: .infinity, minHeight: 400, maxHeight: .infinity)
    }
}

struct RenderControlsView: View {
    
    let onUpdateClicked: ((Float, Float, Float, Float, Float)) -> Void;
    
    @State var exposure: Double = 0
    
    @State var curve0: Double = 0
    @State var curve1: Double = 0
    @State var curve2: Double = 0
    @State var curve3: Double = 0
    @State var curve4: Double = 0
    
    var body: some View {
        VStack {
            HStack {
                Text("Baseline Exposure: \(self.exposure)")
                SlideyBoi(value: $exposure, min:-5, max:5)
            }
            Text("Tone Curve")
            HStack {
                SlideyBoi(value: $curve0, vertical: true, min:-5, max:5)
                SlideyBoi(value: $curve1, vertical: true, min:-5, max:5)
                SlideyBoi(value: $curve2, vertical: true, min:-5, max:5)
                SlideyBoi(value: $curve3, vertical: true, min:-5, max:5)
                SlideyBoi(value: $curve4, vertical: true, min:-5, max:5)
            }
            Button(action: { self.onUpdateClicked((Float(self.curve0), Float(self.curve1), Float(self.curve2), Float(self.curve3), Float(self.curve4))) }){
                Text("Render!")
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



struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView().environmentObject(Workspace());
    }
}
