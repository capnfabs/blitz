import 'dart:ffi';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';

typedef addition_func = Int32 Function(Int32 a, Int32 b);
typedef AdditionFunc = int Function(int a, int b);

typedef raw_renderer_new_func = Pointer<Void> Function(
    Pointer<Utf8> filename);
typedef RawRendererNew = RawRenderer Function(String);

typedef load_preview_func = Pointer<Buffer> Function(Pointer<Void> renderer);
typedef LoadPreview = Uint8List Function(RawRenderer);

class RawRenderer {
  Pointer<Void> renderer;
  RawRenderer(this.renderer);
}

class Api {
  final AdditionFunc addition;
  final RawRendererNew newRenderer;
  final LoadPreview loadPreview;
  const Api(this.addition, this.newRenderer, this.loadPreview);
}

class Buffer extends Struct {
  Pointer<Uint8> data;
  @IntPtr()
  int len;
}

Api getApi() {
  // Open the dynamic library
  final path = 'libblitzexport.dylib';
  final dylib = DynamicLibrary.open(path);

  final addition =
      dylib.lookup<NativeFunction<addition_func>>("addition").asFunction<AdditionFunc>();


  final newRendererNative = dylib
      .lookup<NativeFunction<raw_renderer_new_func>>('raw_renderer_new')
      .asFunction<raw_renderer_new_func>();

  final newRenderer = (String filename) {
    return RawRenderer(newRendererNative(Utf8.toUtf8(filename)));
  };

  final loadPreviewRaw = dylib.lookup<NativeFunction<load_preview_func>>('raw_renderer_get_preview').asFunction<load_preview_func>();

  final loadPreview = (RawRenderer renderer) {
    final bufraw = loadPreviewRaw(renderer.renderer);
    final buffer = bufraw.ref;
    return buffer.data.asTypedList(buffer.len);
  };

  return Api(
    addition,
    newRenderer,
    loadPreview,
  );
}
