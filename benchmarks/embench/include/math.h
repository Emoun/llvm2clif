/* math.h stub for the Embench sources that include it. There is no floating
   point on Scry: huffbench includes the header without using it, and wikisort
   (which calls sqrt) is rejected by llvm2clif. */
#ifndef EMBENCH_SCRY_MATH_H
#define EMBENCH_SCRY_MATH_H
double sqrt(double x);
double fabs(double x);
float fabsf(float x);
#endif
