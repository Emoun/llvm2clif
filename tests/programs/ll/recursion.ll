source_filename = "recursion.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

; Function Attrs: nofree nosync nounwind memory(none)
define dso_local i32 @test(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
  %5 = tail call fastcc i32 @fib(i32 noundef %0)
  br label %tailrecurse.i

tailrecurse.i:                                    ; preds = %6, %4
  %.tr.i = phi i32 [ %0, %4 ], [ %7, %6 ]
  switch i32 %.tr.i, label %6 [
    i32 0, label %is_even.exit
    i32 1, label %is_even.exit.loopexit
  ]

6:                                                ; preds = %tailrecurse.i
  %7 = add nsw i32 %.tr.i, -2
  br label %tailrecurse.i

is_even.exit.loopexit:                            ; preds = %tailrecurse.i
  br label %is_even.exit

is_even.exit:                                     ; preds = %tailrecurse.i, %is_even.exit.loopexit
  %8 = phi i32 [ 0, %is_even.exit.loopexit ], [ 1, %tailrecurse.i ]
  %9 = and i32 %0, 3
  %10 = tail call fastcc i32 @ack(i32 noundef 2, i32 noundef %9)
  %11 = mul nsw i32 %0, 6
  %12 = add nsw i32 %11, 12
  %13 = shl nsw i32 %1, 2
  %14 = add nsw i32 %13, 18
  br label %tailrecurse.i5

tailrecurse.i5:                                   ; preds = %tailrecurse.i5, %is_even.exit
  %.tr57.i = phi i32 [ %15, %tailrecurse.i5 ], [ %14, %is_even.exit ]
  %.tr6.i = phi i32 [ %.tr57.i, %tailrecurse.i5 ], [ %12, %is_even.exit ]
  %15 = srem i32 %.tr6.i, %.tr57.i
  %16 = icmp eq i32 %15, 0
  br i1 %16, label %gcd.exit, label %tailrecurse.i5

gcd.exit:                                         ; preds = %tailrecurse.i5
  %17 = mul nsw i32 %5, 10
  %18 = or disjoint i32 %8, %17
  %19 = mul nsw i32 %10, 100
  %20 = add nsw i32 %18, %19
  %21 = add nsw i32 %20, %.tr57.i
  ret i32 %21
}

; Function Attrs: nofree nosync nounwind memory(none)
define internal fastcc i32 @fib(i32 noundef %0) unnamed_addr #0 {
  %2 = icmp slt i32 %0, 2
  br i1 %2, label %tailrecurse._crit_edge, label %tailrecurse

tailrecurse:                                      ; preds = %1, %tailrecurse
  %.tr5 = phi i32 [ %5, %tailrecurse ], [ %0, %1 ]
  %accumulator.tr4 = phi i32 [ %6, %tailrecurse ], [ 0, %1 ]
  %3 = add nsw i32 %.tr5, -1
  %4 = tail call fastcc i32 @fib(i32 noundef %3)
  %5 = add nsw i32 %.tr5, -2
  %6 = add nsw i32 %4, %accumulator.tr4
  %7 = icmp ult i32 %.tr5, 4
  br i1 %7, label %tailrecurse._crit_edge, label %tailrecurse

tailrecurse._crit_edge:                           ; preds = %tailrecurse, %1
  %accumulator.tr.lcssa = phi i32 [ 0, %1 ], [ %6, %tailrecurse ]
  %.tr.lcssa = phi i32 [ %0, %1 ], [ %5, %tailrecurse ]
  %accumulator.ret.tr = add nsw i32 %.tr.lcssa, %accumulator.tr.lcssa
  ret i32 %accumulator.ret.tr
}

; Function Attrs: nofree nosync nounwind memory(none)
define internal fastcc i32 @ack(i32 noundef %0, i32 noundef %1) unnamed_addr #0 {
  %3 = icmp eq i32 %0, 0
  br i1 %3, label %tailrecurse._crit_edge, label %.lr.ph

tailrecurse._crit_edge:                           ; preds = %tailrecurse.backedge, %2
  %.tr10.lcssa = phi i32 [ %1, %2 ], [ %.tr10.be, %tailrecurse.backedge ]
  %4 = add nsw i32 %.tr10.lcssa, 1
  ret i32 %4

.lr.ph:                                           ; preds = %2, %tailrecurse.backedge
  %.tr1012 = phi i32 [ %.tr10.be, %tailrecurse.backedge ], [ %1, %2 ]
  %.tr11 = phi i32 [ %.tr.be, %tailrecurse.backedge ], [ %0, %2 ]
  %5 = icmp eq i32 %.tr1012, 0
  br i1 %5, label %tailrecurse.backedge, label %7

tailrecurse.backedge:                             ; preds = %.lr.ph, %7
  %.tr10.be = phi i32 [ %9, %7 ], [ 1, %.lr.ph ]
  %.tr.be = add nsw i32 %.tr11, -1
  %6 = icmp eq i32 %.tr.be, 0
  br i1 %6, label %tailrecurse._crit_edge, label %.lr.ph

7:                                                ; preds = %.lr.ph
  %8 = add nsw i32 %.tr1012, -1
  %9 = tail call fastcc i32 @ack(i32 noundef %.tr11, i32 noundef %8)
  br label %tailrecurse.backedge
}

attributes #0 = { nofree nosync nounwind memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }

!llvm.module.flags = !{!0, !1, !2}
!llvm.ident = !{!3}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 1, !"target-abi", !"ilp32"}
!2 = !{i32 8, !"SmallDataLimit", i32 8}
!3 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
