#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct RawRenderer RawRenderer;

typedef struct {
  uint8_t *data;
  uintptr_t len;
} Buffer;

typedef struct {
  float tone_curve[5];
} RenderSettings;

void free_buffer(Buffer buf);

void raw_renderer_free(RawRenderer *ptr);

Buffer raw_renderer_get_preview(RawRenderer *ptr);

RawRenderer *raw_renderer_new(const char *filename);

Buffer raw_renderer_render_image(RawRenderer *ptr);

Buffer raw_renderer_render_with_settings(RawRenderer *ptr, RenderSettings settings);
