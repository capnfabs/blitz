#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef enum {
  Rgb,
  Rgba,
} ImageFormat;

typedef struct RawRenderer RawRenderer;

typedef struct {
  uint8_t *data;
  uintptr_t len;
} Buffer;

typedef struct {
  Buffer data;
  uint32_t width;
  uint32_t height;
  ImageFormat pixel_format;
} RawImage;

typedef struct {
  RawImage img;
  RawImage histogram;
} ImageAndHistogram;

typedef struct {
  float tone_curve[5];
  float exposure_basis;
  bool auto_contrast;
  float saturation_boost;
} RenderSettings;

void free_buffer(Buffer buf);

void raw_renderer_free(RawRenderer *ptr);

Buffer raw_renderer_get_preview(RawRenderer *ptr);

RawRenderer *raw_renderer_new(const char *filename);

RawImage raw_renderer_render_image(RawRenderer *ptr);

ImageAndHistogram raw_renderer_render_with_settings(RawRenderer *ptr, RenderSettings settings);
