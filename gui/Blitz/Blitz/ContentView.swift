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
                Text("Hi there! Please open a file.")
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                Image(nsImage: workspace.preview!)
                    .frame(width: 300, height: 300)
            }
        }
    }
}


struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView().environmentObject(Workspace());
    }
}
