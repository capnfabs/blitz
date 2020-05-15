#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct RawRenderer RawRenderer;

typedef struct {
  uint8_t *data;
  uintptr_t len;
} Buffer;

void free_buffer(Buffer buf);

void raw_renderer_free(RawRenderer *ptr);

Buffer raw_renderer_get_preview(RawRenderer *ptr);

RawRenderer *raw_renderer_new(const char *filename);
