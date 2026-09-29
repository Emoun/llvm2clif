source_filename = "compare.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none)
define dso_local i32 @test(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
  %5 = icmp slt i32 %0, %1
  %6 = zext i1 %5 to i32
  %.not = icmp sgt i32 %0, %1
  %7 = select i1 %.not, i32 0, i32 2
  %8 = select i1 %.not, i32 4, i32 0
  %9 = select i1 %5, i32 0, i32 8
  %10 = icmp eq i32 %0, %1
  %11 = select i1 %10, i32 16, i32 0
  %12 = select i1 %10, i32 0, i32 32
  %13 = icmp ult i32 %0, %1
  %14 = select i1 %13, i32 64, i32 0
  %15 = icmp ugt i32 %0, %1
  %16 = select i1 %15, i32 128, i32 0
  %17 = select i1 %15, i32 0, i32 256
  %18 = select i1 %13, i32 0, i32 512
  %19 = tail call i32 @llvm.smin.i32(i32 %2, i32 %3)
  %20 = mul nsw i32 %19, 3
  %21 = tail call i32 @llvm.smax.i32(i32 %2, i32 %3)
  %22 = mul nsw i32 %21, 5
  %23 = and i32 %1, %0
  %24 = lshr i32 %23, 21
  %25 = and i32 %24, 1024
  %26 = or i32 %1, %0
  %27 = lshr i32 %26, 20
  %28 = and i32 %27, 2048
  %.unshifted = xor i32 %1, %0
  %29 = lshr i32 %.unshifted, 19
  %30 = and i32 %29, 4096
  %31 = lshr i32 %0, 18
  %32 = and i32 %31, 8192
  %33 = xor i32 %32, 8192
  %sext = shl i32 %0, 24
  %sext69 = shl i32 %1, 24
  %34 = icmp slt i32 %sext, %sext69
  %35 = select i1 %34, i32 16384, i32 0
  %36 = and i32 %2, 255
  %37 = icmp ugt i32 %36, 100
  %38 = select i1 %37, i32 32768, i32 0
  %39 = or disjoint i32 %7, %6
  %40 = or disjoint i32 %39, %8
  %41 = or disjoint i32 %40, %9
  %42 = or disjoint i32 %41, %11
  %43 = or disjoint i32 %42, %12
  %44 = or disjoint i32 %43, %14
  %45 = add nuw nsw i32 %44, %16
  %46 = add nuw nsw i32 %45, %17
  %47 = add nuw nsw i32 %46, %18
  %48 = add nuw nsw i32 %47, %33
  %49 = add nuw nsw i32 %48, %25
  %50 = add nuw nsw i32 %49, %28
  %51 = add nuw nsw i32 %50, %30
  %52 = add nuw nsw i32 %51, %35
  %53 = add i32 %52, %20
  %54 = add i32 %53, %22
  %55 = add i32 %54, %38
  ret i32 %55
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.smin.i32(i32, i32) #1

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.smax.i32(i32, i32) #1

attributes #0 = { mustprogress nofree norecurse nosync nounwind willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #1 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }

!llvm.module.flags = !{!0, !1, !2}
!llvm.ident = !{!3}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 1, !"target-abi", !"ilp32"}
!2 = !{i32 8, !"SmallDataLimit", i32 8}
!3 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
