/* assert.h for scry-cc programs: a failed assertion calls abort(), which
   traps in the simulator. There is no way to print the message. */
#ifndef SCRY_ASSERT_H
#define SCRY_ASSERT_H

#include <stdlib.h>

#ifdef NDEBUG
#define assert(e) ((void)0)
#else
#define assert(e) ((e) ? (void)0 : abort())
#endif

#endif
