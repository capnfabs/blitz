import 'dart:ffi';
import 'package:ffi/ffi.dart';

typedef addition_func = Int32 Function(Int32 a, Int32 b);
typedef AdditionFunc = int Function(int a, int b);

class RawRenderer extends Struct {}

typedef raw_renderer_new_func = Pointer<RawRenderer> Function(
    Pointer<Utf8> filename);

class Api {
  final AdditionFunc addition;
  final raw_renderer_new_func newRenderer;
  const Api(this.addition, this.newRenderer);
}

Api getApi() {
  // Open the dynamic library
  final path = 'libblitzexport.dylib';
  final dylib = DynamicLibrary.open(path);

  final addition =
      dylib.lookup<NativeFunction<addition_func>>("addition").asFunction<AdditionFunc>();


  final newRenderer = dylib
      .lookup<NativeFunction<raw_renderer_new_func>>('raw_renderer_new')
      .asFunction<raw_renderer_new_func>();

  return Api(
    addition,
    newRenderer,
  );
}
