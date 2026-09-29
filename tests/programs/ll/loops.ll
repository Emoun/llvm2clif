source_filename = "loops.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

; Function Attrs: nofree norecurse nosync nounwind memory(none)
define dso_local i32 @test(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
.preheader27.preheader:
  %4 = add nsw i32 %0, 1
  br label %5

5:                                                ; preds = %.preheader27.preheader, %5
  %.0 = phi i32 [ %6, %5 ], [ 0, %.preheader27.preheader ]
  %6 = add nsw i32 %4, %.0
  %7 = icmp slt i32 %6, 100
  br i1 %7, label %5, label %8, !llvm.loop !4

8:                                                ; preds = %5
  %.not7.i = icmp slt i32 %0, 2
  br i1 %.not7.i, label %fact.exit, label %.lr.ph.i

.lr.ph.i:                                         ; preds = %8, %.lr.ph.i
  %.09.i = phi i32 [ %10, %.lr.ph.i ], [ 2, %8 ]
  %.068.i = phi i32 [ %9, %.lr.ph.i ], [ 1, %8 ]
  %9 = mul nsw i32 %.068.i, %.09.i
  %10 = add nuw i32 %.09.i, 1
  %exitcond.not.i = icmp eq i32 %.09.i, %0
  br i1 %exitcond.not.i, label %fact.exit, label %.lr.ph.i, !llvm.loop !6

fact.exit:                                        ; preds = %.lr.ph.i, %8
  %.06.lcssa.i = phi i32 [ 1, %8 ], [ %9, %.lr.ph.i ]
  %11 = icmp sgt i32 %0, 0
  br i1 %11, label %.lr.ph.i26, label %fib.exit

.lr.ph.i26:                                       ; preds = %fact.exit, %.lr.ph.i26
  %.010.i = phi i32 [ %12, %.lr.ph.i26 ], [ %0, %fact.exit ]
  %.069.i = phi i32 [ %13, %.lr.ph.i26 ], [ 1, %fact.exit ]
  %.078.i = phi i32 [ %.069.i, %.lr.ph.i26 ], [ 0, %fact.exit ]
  %12 = add nsw i32 %.010.i, -1
  %13 = add nsw i32 %.078.i, %.069.i
  %14 = icmp ugt i32 %.010.i, 1
  br i1 %14, label %.lr.ph.i26, label %fib.exit.loopexit, !llvm.loop !7

fib.exit.loopexit:                                ; preds = %.lr.ph.i26
  %15 = mul nsw i32 %.069.i, 3
  br label %fib.exit

fib.exit:                                         ; preds = %fib.exit.loopexit, %fact.exit
  %.07.lcssa.i = phi i32 [ 0, %fact.exit ], [ %15, %fib.exit.loopexit ]
  %16 = sub nsw i32 0, %1
  %17 = sub nsw i32 36, %1
  %18 = sub nsw i32 64, %1
  %19 = sub nsw i32 100, %1
  %20 = sub nsw i32 144, %1
  %21 = sub nsw i32 196, %1
  %.neg = mul i32 %1, -3
  %22 = add i32 %.neg, %17
  %23 = shl i32 %1, 1
  %24 = sub i32 %22, %23
  %25 = add i32 %24, 459
  %26 = xor i32 %21, %25
  %27 = xor i32 %20, %26
  %28 = xor i32 %19, %27
  %29 = xor i32 %18, %28
  %30 = sub nsw i32 16, %1
  %31 = xor i32 %29, %30
  %32 = sub nsw i32 4, %1
  %33 = xor i32 %31, %32
  %34 = xor i32 %33, %16
  %35 = xor i32 %34, %17
  %36 = add i32 %6, %35
  %37 = add i32 %36, %.06.lcssa.i
  %38 = add i32 %37, %.07.lcssa.i
  ret i32 %38
}

attributes #0 = { nofree norecurse nosync nounwind memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }

!llvm.module.flags = !{!0, !1, !2}
!llvm.ident = !{!3}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 1, !"target-abi", !"ilp32"}
!2 = !{i32 8, !"SmallDataLimit", i32 8}
!3 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
!4 = distinct !{!4, !5}
!5 = !{!"llvm.loop.mustprogress"}
!6 = distinct !{!6, !5}
!7 = distinct !{!7, !5}
