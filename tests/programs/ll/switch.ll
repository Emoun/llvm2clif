source_filename = "switch.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

@switch.table.test = private unnamed_addr constant [9 x i32] [i32 10000, i32 11000, i32 12000, i32 13000, i32 14000, i32 15000, i32 -1000, i32 17000, i32 18000], align 4
@switch.table.test.2 = private unnamed_addr constant [3 x i32] [i32 11, i32 10, i32 1100], align 4

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none)
define dso_local i32 @test(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
  %5 = icmp ult i32 %0, 9
  br i1 %5, label %switch.lookup, label %dense.exit

switch.lookup:                                    ; preds = %4
  %switch.gep = getelementptr inbounds [9 x i32], ptr @switch.table.test, i32 0, i32 %0
  %switch.load = load i32, ptr %switch.gep, align 4
  br label %dense.exit

dense.exit:                                       ; preds = %4, %switch.lookup
  %.0.i = phi i32 [ %switch.load, %switch.lookup ], [ -1000, %4 ]
  switch i32 %1, label %10 [
    i32 7, label %sparse.exit
    i32 100, label %6
    i32 1000, label %7
    i32 -1, label %8
    i32 65536, label %9
  ]

6:                                                ; preds = %dense.exit
  br label %sparse.exit

7:                                                ; preds = %dense.exit
  br label %sparse.exit

8:                                                ; preds = %dense.exit
  br label %sparse.exit

9:                                                ; preds = %dense.exit
  br label %sparse.exit

10:                                               ; preds = %dense.exit
  br label %sparse.exit

sparse.exit:                                      ; preds = %dense.exit, %6, %7, %8, %9, %10
  %.0.i4 = phi i32 [ 0, %10 ], [ 500, %9 ], [ 400, %8 ], [ 300, %7 ], [ 200, %6 ], [ 100, %dense.exit ]
  %11 = and i32 %0, 3
  %.not = icmp eq i32 %11, 3
  br i1 %.not, label %fallthrough.exit, label %switch.lookup7

switch.lookup7:                                   ; preds = %sparse.exit
  %switch.gep8 = getelementptr inbounds [3 x i32], ptr @switch.table.test.2, i32 0, i32 %11
  %switch.load9 = load i32, ptr %switch.gep8, align 4
  br label %fallthrough.exit

fallthrough.exit:                                 ; preds = %sparse.exit, %switch.lookup7
  %.2.i = phi i32 [ %switch.load9, %switch.lookup7 ], [ 1000, %sparse.exit ]
  %12 = and i32 %1, 3
  %.not13 = icmp eq i32 %12, 3
  br i1 %.not13, label %fallthrough.exit6, label %switch.lookup10

switch.lookup10:                                  ; preds = %fallthrough.exit
  %switch.gep11 = getelementptr inbounds [3 x i32], ptr @switch.table.test.2, i32 0, i32 %12
  %switch.load12 = load i32, ptr %switch.gep11, align 4
  br label %fallthrough.exit6

fallthrough.exit6:                                ; preds = %fallthrough.exit, %switch.lookup10
  %.2.i5 = phi i32 [ %switch.load12, %switch.lookup10 ], [ 1000, %fallthrough.exit ]
  %13 = add nsw i32 %.0.i4, %.0.i
  %14 = add nsw i32 %13, %.2.i
  %15 = add nsw i32 %14, %.2.i5
  ret i32 %15
}

attributes #0 = { mustprogress nofree norecurse nosync nounwind willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }

!llvm.module.flags = !{!0, !1, !2}
!llvm.ident = !{!3}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 1, !"target-abi", !"ilp32"}
!2 = !{i32 8, !"SmallDataLimit", i32 8}
!3 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
