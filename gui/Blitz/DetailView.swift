//
//  DetailView.swift
//  Blitz
//
//  Created by Fabian Tamp on 16/9/20.
//  Copyright © 2020 Fabian Tamp. All rights reserved.
//

import SwiftUI

struct DetailView: View {
    let filename: String
    
    @ObservedObject var image: AsyncImage;
    
    // TODO: Something to make this size consistently regardless of whether there's already a view or not.
    var body: some View {
        let img = displayImage()
        let imgIsOld = image.image == nil
        return VStack {
            Group {
                if img != nil {
                    HStack {
                        ZStack {
                            Image(nsImage: img!)
                                .resizable()
                                .aspectRatio(contentMode: .fit)
                            if imgIsOld {
                                // TODO; this is side-effect-y
                                Text("Rerendering...")
                                .padding(20)
                                    .foregroundColor(.white)
                                    .background(Color.black)
                            }
                        }
                        VStack {
                            Button(action: {
                                let appDelegate = NSApplication.shared.delegate as! AppDelegate
                                appDelegate.saveRender(label: self.filename, data: self.image.image!)
                            }) {
                                Text("Save")
                            }
                        }.disabled(imgIsOld)
                    }
                } else {
                    Text("Click render to start.")
                    .padding(20)
                }
            }
            if self.image.histogram != nil {
                Image(nsImage: self.image.histogram!)
            }
            RenderControlsView(onUpdateClicked: {
                self.image.loadWithSettings(settings: $0)
            })
        }
        .frame(minWidth: 400, maxWidth: .infinity, minHeight: 400, maxHeight: .infinity)
    }
    
    func displayImage() -> NSImage? {
        image.image ?? image.lastImage
    }
}

struct RenderControlsView: View {
    
    let onUpdateClicked: (RenderSettings) -> Void;
    
    @State var exposure: Double = 0
    
    @State var curve0: Double = 0
    @State var curve1: Double = 0
    @State var curve2: Double = 0
    @State var curve3: Double = 0
    @State var curve4: Double = 0
    
    var body: some View {
        VStack {
            Button(action: {
                let tone_curve = (Float(self.curve0), Float(self.curve1), Float(self.curve2), Float(self.curve3), Float(self.curve4))
                let rs = RenderSettings(tone_curve: tone_curve, exposure_basis: Float(self.exposure))
                self.onUpdateClicked(rs)
                
            }){
                Text("Render!")
            }
            HStack {
                Text("Baseline Exposure: \(self.exposure)")
                SlideyBoi(value: $exposure, min:-5, max:5)
            }
            HStack {
                Text("Tone Curve")
                Button(action: {
                    self.curve0 = 0
                    self.curve1 = 0
                    self.curve2 = 0
                    self.curve3 = 0
                    self.curve4 = 0
                }) {
                    Text("Reset")
                }.buttonStyle(InlineButtonStyle())
            }
            HStack {
                SlideyBoi(value: $curve0, vertical: true, min:-5, max:5)
                SlideyBoi(value: $curve1, vertical: true, min:-5, max:5)
                SlideyBoi(value: $curve2, vertical: true, min:-5, max:5)
                SlideyBoi(value: $curve3, vertical: true, min:-5, max:5)
                SlideyBoi(value: $curve4, vertical: true, min:-5, max:5)
            }
        }
    }
}

