source_filename = "bitfields.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none)
define dso_local i32 @test(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
  %5 = and i32 %0, 7
  %6 = sub nsw i32 %0, %1
  %7 = mul nsw i32 %1, %0
  %8 = and i32 %7, 65535
  %9 = mul nsw i32 %0, 1000
  %10 = add nsw i32 %9, %1
  %.sroa.45.0.extract.shift = lshr i32 %10, 16
  %sum.shift = lshr i32 %10, 24
  %11 = add nsw i32 %1, %0
  %12 = and i32 %1, 31
  %13 = mul nuw nsw i32 %12, 10
  %14 = shl i32 %6, 24
  %15 = ashr exact i32 %14, 24
  %16 = mul nsw i32 %15, 100
  %17 = and i32 %10, 255
  %18 = trunc i32 %sum.shift to i16
  %.sroa.45.sroa.4.0.insert.shift = shl nuw i16 %18, 8
  %19 = trunc i32 %.sroa.45.0.extract.shift to i16
  %.sroa.45.sroa.0.0.insert.ext = and i16 %19, 255
  %.sroa.45.sroa.0.0.insert.insert = or disjoint i16 %.sroa.45.sroa.4.0.insert.shift, %.sroa.45.sroa.0.0.insert.ext
  %20 = sext i16 %.sroa.45.sroa.0.0.insert.insert to i32
  %sext = shl i32 %0, 24
  %21 = ashr exact i32 %sext, 24
  %sext27 = shl i32 %11, 16
  %22 = ashr exact i32 %sext27, 16
  %23 = add i32 %5, %1
  %24 = add i32 %23, %21
  %25 = add i32 %24, %13
  %26 = add i32 %25, %8
  %27 = add i32 %26, %sum.shift
  %28 = add i32 %27, %17
  %29 = add i32 %28, %22
  %30 = add i32 %29, %16
  %31 = add i32 %30, %20
  ret i32 %31
}

attributes #0 = { mustprogress nofree norecurse nosync nounwind willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }

!llvm.module.flags = !{!0, !1, !2}
!llvm.ident = !{!3}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 1, !"target-abi", !"ilp32"}
!2 = !{i32 8, !"SmallDataLimit", i32 8}
!3 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
