/* Board support for running Embench on the scryer simulator: nothing to
   initialise, and the triggers are only markers. */
#include <support.h>

void initialise_board(void) {}
void __attribute__((noinline)) start_trigger(void) {}
void __attribute__((noinline)) stop_trigger(void) {}
