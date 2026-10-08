source_filename = "boolsext.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none)
define dso_local i32 @test(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
  %5 = icmp slt i32 %0, %1
  %6 = sext i1 %5 to i32
  %7 = select i1 %5, i32 4660, i32 0
  %8 = icmp sgt i32 %0, %1
  %9 = select i1 %8, i32 -2, i32 0
  %.lobit = ashr i32 %0, 31
  %10 = icmp sgt i32 %1, -1
  %11 = zext i1 %10 to i32
  %12 = xor i32 %.lobit, %11
  %13 = trunc i32 %2 to i16
  %14 = icmp ugt i16 %13, 255
  %15 = icmp sgt i16 %13, -1
  %spec.select.i = sext i1 %15 to i32
  %.0.i = select i1 %14, i32 %spec.select.i, i32 %2
  %16 = and i32 %.0.i, 255
  %17 = trunc i32 %3 to i16
  %18 = mul i16 %17, 3
  %19 = add i16 %18, -200
  %20 = icmp ugt i16 %19, 255
  %21 = zext nneg i16 %19 to i32
  %22 = icmp sgt i16 %19, -1
  %spec.select.i27 = sext i1 %22 to i32
  %.0.i28 = select i1 %20, i32 %spec.select.i27, i32 %21
  %23 = shl nsw i32 %.0.i28, 1
  %24 = and i32 %23, 510
  %25 = shl nsw i32 %6, 1
  %.not = icmp eq i32 %0, 0
  %26 = select i1 %.not, i32 0, i32 15
  %27 = add nsw i32 %9, %26
  %28 = add nsw i32 %27, %7
  %29 = add nsw i32 %28, %12
  %30 = add nsw i32 %29, %25
  %31 = add nsw i32 %30, %16
  %32 = add i32 %31, %24
  ret i32 %32
}

attributes #0 = { mustprogress nofree norecurse nosync nounwind willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }

!llvm.module.flags = !{!0, !1, !2}
!llvm.ident = !{!3}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 1, !"target-abi", !"ilp32"}
!2 = !{i32 8, !"SmallDataLimit", i32 8}
!3 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
