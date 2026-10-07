source_filename = "rettag.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none)
define dso_local i32 @test(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
  %5 = tail call fastcc i32 @sort4(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3)
  %6 = lshr i32 %5, 1
  %7 = icmp ult i32 %5, 5
  %8 = zext i1 %7 to i32
  %9 = add nuw i32 %6, %8
  %10 = udiv i32 %5, 3
  %11 = urem i32 %10, 1000
  %12 = add nuw i32 %9, %11
  %13 = tail call fastcc i32 @sort4(i32 noundef %3, i32 noundef %2, i32 noundef %1, i32 noundef %0)
  %14 = lshr i32 %13, 3
  %15 = and i32 %14, 65535
  %16 = add nuw i32 %12, %15
  ret i32 %16
}

; Function Attrs: mustprogress nofree noinline norecurse nosync nounwind willreturn memory(none)
define internal fastcc i32 @sort4(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) unnamed_addr #1 {
.lr.ph.preheader:
  %spec.select = tail call i32 @llvm.smax.i32(i32 %0, i32 %1)
  %spec.select36 = tail call i32 @llvm.smin.i32(i32 %0, i32 %1)
  %.sroa.22.3 = tail call i32 @llvm.smax.i32(i32 %spec.select, i32 %2)
  %.sroa.10.7 = tail call i32 @llvm.smin.i32(i32 %spec.select, i32 %2)
  %.sroa.30.1 = tail call i32 @llvm.smax.i32(i32 %.sroa.22.3, i32 %3)
  %.sroa.22.4 = tail call i32 @llvm.smin.i32(i32 %.sroa.22.3, i32 %3)
  %4 = icmp sgt i32 %spec.select36, %2
  %.sroa.10.1 = select i1 %4, i32 %spec.select36, i32 %.sroa.10.7
  %.sroa.0.1 = select i1 %4, i32 %.sroa.10.7, i32 %spec.select36
  %.sroa.22.1 = tail call i32 @llvm.smax.i32(i32 %.sroa.10.1, i32 %.sroa.22.4)
  %.sroa.10.2 = tail call i32 @llvm.smin.i32(i32 %.sroa.10.1, i32 %.sroa.22.4)
  %.sroa.10.4 = tail call i32 @llvm.smax.i32(i32 %.sroa.0.1, i32 %.sroa.10.2)
  %.sroa.0.3 = tail call i32 @llvm.smin.i32(i32 %.sroa.0.1, i32 %.sroa.10.2)
  %5 = mul nsw i32 %.sroa.0.3, 1000
  %6 = mul nsw i32 %.sroa.10.4, 100
  %7 = add nsw i32 %6, %5
  %8 = mul nsw i32 %.sroa.22.1, 10
  %9 = add nsw i32 %7, %8
  %10 = add nsw i32 %9, %.sroa.30.1
  ret i32 %10
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.smax.i32(i32, i32) #2

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.smin.i32(i32, i32) #2

attributes #0 = { mustprogress nofree norecurse nosync nounwind willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #1 = { mustprogress nofree noinline norecurse nosync nounwind willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #2 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }

!llvm.module.flags = !{!0, !1, !2}
!llvm.ident = !{!3}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 1, !"target-abi", !"ilp32"}
!2 = !{i32 8, !"SmallDataLimit", i32 8}
!3 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
