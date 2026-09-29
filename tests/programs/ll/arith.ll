source_filename = "arith.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none)
define dso_local i32 @test(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
  %5 = add i32 %1, 2
  %6 = mul i32 %5, %0
  %.not = icmp eq i32 %1, 0
  br i1 %.not, label %19, label %7

7:                                                ; preds = %4
  %8 = icmp eq i32 %0, -2147483648
  %9 = icmp eq i32 %1, -1
  %or.cond = and i1 %8, %9
  br i1 %or.cond, label %19, label %10

10:                                               ; preds = %7
  %11 = sdiv i32 %0, %1
  %12 = add nsw i32 %11, %6
  %13 = mul i32 %11, %1
  %.decomposed = sub i32 %0, %13
  %14 = add nsw i32 %12, %.decomposed
  %15 = udiv i32 %0, %1
  %16 = add nsw i32 %14, %15
  %17 = mul i32 %15, %1
  %.decomposed66 = sub i32 %0, %17
  %18 = add nsw i32 %16, %.decomposed66
  br label %19

19:                                               ; preds = %7, %10, %4
  %.0 = phi i32 [ %6, %7 ], [ %18, %10 ], [ %6, %4 ]
  %20 = ashr i32 %0, 3
  %21 = lshr i32 %0, 5
  %22 = shl i32 %0, 2
  %23 = xor i32 %1, %0
  %24 = and i32 %0, 65535
  %sext = shl i32 %1, 16
  %25 = ashr exact i32 %sext, 16
  %26 = and i32 %0, 255
  %sext64 = shl i32 %1, 24
  %27 = ashr exact i32 %sext64, 24
  %28 = mul nuw nsw i32 %24, 3
  %29 = lshr i32 %28, 1
  %30 = mul nsw i32 %27, %26
  %31 = add i32 %0, 200
  %32 = and i32 %31, 255
  %sext65 = add i32 %sext, -1966080000
  %33 = ashr exact i32 %sext65, 16
  %34 = add nsw i32 %24, -1
  %35 = add nsw i32 %34, %26
  %36 = add nsw i32 %35, %20
  %37 = add nsw i32 %36, %21
  %38 = add i32 %37, %22
  %39 = add i32 %38, %23
  %40 = add i32 %39, %32
  %41 = add i32 %40, %25
  %42 = add i32 %41, %27
  %43 = add i32 %42, %29
  %44 = add i32 %43, %30
  %45 = add i32 %44, %33
  %46 = add i32 %45, %.0
  ret i32 %46
}

attributes #0 = { mustprogress nofree norecurse nosync nounwind willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }

!llvm.module.flags = !{!0, !1, !2}
!llvm.ident = !{!3}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 1, !"target-abi", !"ilp32"}
!2 = !{i32 8, !"SmallDataLimit", i32 8}
!3 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
