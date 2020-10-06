#include <stdlib.h>
#include "svdpi.h"
#include "Vmain__Dpi.h"

const char* futil_getenv(const char* env_var) {
  const char* out = getenv(env_var);
  if (out == NULL) {
    return "";
  }
  return out;
}
