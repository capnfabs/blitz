//
//  RenderCache.swift
//  Blitz
//
//  Created by Fabian Tamp on 18/5/20.
//  Copyright © 2020 Fabian Tamp. All rights reserved.
//

import Foundation

class RenderCache {
    
    var storage : [URL: Data] = [:]
    
    func load_or(key: URL, creator: (() -> Data)) -> Data {
        if let val = self.storage[key] {
            return val
        } else {
            let val = creator()
            self.storage[key] = val
            return val
        }
    }
}
