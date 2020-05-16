//
//  ContentView.swift
//  Blitz
//
//  Created by Fabian Tamp on 15/5/20.
//  Copyright © 2020 Fabian Tamp. All rights reserved.
//

import SwiftUI

struct ContentView: View {
    
    @EnvironmentObject var workspace: Workspace;
    
    var body: some View {
        VStack {
            if !workspace.loaded {
                Text("Hi there! Please choose a directory.")
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                Text("Opened directory \(workspace.directory!)")
                List(workspace.previews, id: \.id) { preview in
                    ImageTile(image: preview)
                }
            }
        }
    }
}


struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView().environmentObject(Workspace());
    }
}
