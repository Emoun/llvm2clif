source_filename = "signedness.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

; Function Attrs: nofree norecurse nosync nounwind memory(none)
define dso_local i32 @test(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
  %5 = alloca [16 x i32], align 4
  %6 = alloca [16 x i8], align 1
  call void @llvm.lifetime.start.p0(i64 64, ptr nonnull %5) #5
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %6) #5
  %7 = shl i32 %0, 3
  %8 = sub i32 %1, %7
  store i32 %8, ptr %5, align 4, !tbaa !4
  %9 = mul i32 %0, -7
  %10 = add i32 %9, %1
  %11 = getelementptr inbounds [16 x i32], ptr %5, i32 0, i32 1
  store i32 %10, ptr %11, align 4, !tbaa !4
  %12 = mul i32 %0, -6
  %13 = add i32 %12, %1
  %14 = getelementptr inbounds [16 x i32], ptr %5, i32 0, i32 2
  store i32 %13, ptr %14, align 4, !tbaa !4
  %15 = mul i32 %0, -5
  %16 = add i32 %15, %1
  %17 = getelementptr inbounds [16 x i32], ptr %5, i32 0, i32 3
  store i32 %16, ptr %17, align 4, !tbaa !4
  %18 = shl i32 %0, 2
  %19 = sub i32 %1, %18
  %20 = getelementptr inbounds [16 x i32], ptr %5, i32 0, i32 4
  store i32 %19, ptr %20, align 4, !tbaa !4
  %21 = mul i32 %0, -3
  %22 = add i32 %21, %1
  %23 = getelementptr inbounds [16 x i32], ptr %5, i32 0, i32 5
  store i32 %22, ptr %23, align 4, !tbaa !4
  %24 = shl i32 %0, 1
  %25 = sub i32 %1, %24
  %26 = getelementptr inbounds [16 x i32], ptr %5, i32 0, i32 6
  store i32 %25, ptr %26, align 4, !tbaa !4
  %27 = sub i32 %1, %0
  %28 = getelementptr inbounds [16 x i32], ptr %5, i32 0, i32 7
  store i32 %27, ptr %28, align 4, !tbaa !4
  %29 = getelementptr inbounds [16 x i32], ptr %5, i32 0, i32 8
  store i32 %1, ptr %29, align 4, !tbaa !4
  %30 = add i32 %0, %1
  %31 = getelementptr inbounds [16 x i32], ptr %5, i32 0, i32 9
  store i32 %30, ptr %31, align 4, !tbaa !4
  %32 = shl i32 %0, 1
  %33 = add i32 %32, %1
  %34 = getelementptr inbounds [16 x i32], ptr %5, i32 0, i32 10
  store i32 %33, ptr %34, align 4, !tbaa !4
  %35 = mul i32 %0, 3
  %36 = add i32 %35, %1
  %37 = getelementptr inbounds [16 x i32], ptr %5, i32 0, i32 11
  store i32 %36, ptr %37, align 4, !tbaa !4
  %38 = shl i32 %0, 2
  %39 = add i32 %38, %1
  %40 = getelementptr inbounds [16 x i32], ptr %5, i32 0, i32 12
  store i32 %39, ptr %40, align 4, !tbaa !4
  %41 = mul i32 %0, 5
  %42 = add i32 %41, %1
  %43 = getelementptr inbounds [16 x i32], ptr %5, i32 0, i32 13
  store i32 %42, ptr %43, align 4, !tbaa !4
  %44 = mul i32 %0, 6
  %45 = add i32 %44, %1
  %46 = getelementptr inbounds [16 x i32], ptr %5, i32 0, i32 14
  store i32 %45, ptr %46, align 4, !tbaa !4
  %47 = mul i32 %0, 7
  %48 = add i32 %47, %1
  %49 = getelementptr inbounds [16 x i32], ptr %5, i32 0, i32 15
  store i32 %48, ptr %49, align 4, !tbaa !4
  %50 = call fastcc i32 @idx_sum(ptr noundef nonnull %5, i32 noundef 16)
  %51 = and i32 %2, 7
  %52 = add nuw nsw i32 %51, 1
  %53 = call fastcc i32 @idx_sum(ptr noundef nonnull %20, i32 noundef %52)
  %54 = tail call fastcc i32 @both_ways(i32 noundef %0, i32 noundef %1)
  %55 = tail call fastcc i32 @both_ways(i32 noundef %1, i32 noundef %2)
  %56 = sub i32 0, %0
  %57 = tail call fastcc i32 @both_ways(i32 noundef %56, i32 noundef %3)
  %58 = and i32 %0, 15
  %59 = call fastcc i32 @count_down(i32 noundef %58, ptr noundef nonnull %6)
  %60 = and i32 %59, 65535
  %61 = getelementptr inbounds [16 x i8], ptr %6, i32 0, i32 3
  %62 = load i8, ptr %61, align 1, !tbaa !8
  %63 = zext i8 %62 to i32
  %64 = getelementptr inbounds [16 x i8], ptr %6, i32 0, i32 7
  %65 = load i8, ptr %64, align 1, !tbaa !8
  %66 = zext i8 %65 to i32
  %sext.mask = and i32 %0, 128
  %.not = icmp eq i32 %sext.mask, 0
  %67 = select i1 %.not, i32 0, i32 100
  %68 = shl i32 %1, 23
  %69 = ashr i32 %68, 26
  %sext = shl i32 %2, 24
  %70 = ashr exact i32 %sext, 24
  %sext35 = shl i32 %3, 24
  %71 = ashr exact i32 %sext35, 24
  %72 = mul nsw i32 %71, %70
  %sext36 = shl i32 %1, 16
  %73 = icmp slt i32 %sext36, -327680
  %74 = select i1 %73, i32 2000, i32 0
  %sext37 = mul i32 %2, 196608
  %75 = ashr i32 %sext37, 21
  %76 = and i32 %0, 255
  %77 = icmp ugt i32 %76, 200
  %78 = select i1 %77, i32 7, i32 0
  %79 = xor i32 %0, -2147483648
  %80 = icmp slt i32 %79, %1
  %81 = select i1 %80, i32 11, i32 13
  %reass.add = add nuw nsw i32 %76, %66
  %reass.mul = mul nuw nsw i32 %reass.add, 3
  %82 = add nsw i32 %67, %69
  %83 = add nsw i32 %82, %78
  %84 = add nsw i32 %83, %81
  %85 = add nsw i32 %84, %75
  %86 = add nsw i32 %85, %74
  %87 = add nsw i32 %86, %72
  %88 = add i32 %87, %50
  %89 = add i32 %88, %53
  %90 = add i32 %89, %54
  %91 = add i32 %90, %55
  %92 = add i32 %91, %57
  %93 = add i32 %92, %60
  %94 = add i32 %93, %63
  %95 = add i32 %94, %reass.mul
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %6) #5
  call void @llvm.lifetime.end.p0(i64 64, ptr nonnull %5) #5
  ret i32 %95
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #1

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #1

; Function Attrs: nofree noinline norecurse nosync nounwind memory(argmem: read)
define internal fastcc i32 @idx_sum(ptr nocapture noundef readonly %0, i32 noundef %1) unnamed_addr #2 {
  %3 = icmp sgt i32 %1, 0
  br i1 %3, label %.lr.ph, label %._crit_edge

.lr.ph:                                           ; preds = %2
  %4 = lshr i32 %1, 1
  br label %5

._crit_edge:                                      ; preds = %5, %2
  %.08.lcssa = phi i32 [ 0, %2 ], [ %10, %5 ]
  ret i32 %.08.lcssa

5:                                                ; preds = %.lr.ph, %5
  %.011 = phi i32 [ 0, %.lr.ph ], [ %11, %5 ]
  %.0810 = phi i32 [ 0, %.lr.ph ], [ %10, %5 ]
  %6 = getelementptr inbounds i32, ptr %0, i32 %.011
  %7 = load i32, ptr %6, align 4, !tbaa !4
  %8 = sub nsw i32 %.011, %4
  %9 = mul i32 %7, %8
  %10 = add i32 %9, %.0810
  %11 = add nuw nsw i32 %.011, 1
  %exitcond.not = icmp eq i32 %11, %1
  br i1 %exitcond.not, label %._crit_edge, label %5, !llvm.loop !9
}

; Function Attrs: mustprogress nofree noinline norecurse nosync nounwind willreturn memory(none)
define internal fastcc i32 @both_ways(i32 noundef %0, i32 noundef %1) unnamed_addr #3 {
  %3 = icmp slt i32 %0, %1
  %spec.select = zext i1 %3 to i32
  %4 = icmp ult i32 %0, %1
  %5 = or disjoint i32 %spec.select, 2
  %.1 = select i1 %4, i32 %5, i32 %spec.select
  %6 = ashr i32 %0, 3
  %7 = icmp eq i32 %6, %1
  %8 = or disjoint i32 %.1, 4
  %.2 = select i1 %7, i32 %8, i32 %.1
  %9 = lshr i32 %0, 3
  %10 = icmp eq i32 %9, %1
  %11 = or disjoint i32 %.2, 8
  %.3 = select i1 %10, i32 %11, i32 %.2
  %12 = sdiv i32 %0, 7
  %13 = shl i32 %12, 4
  %14 = mul i32 %12, 7
  %.decomposed = sub i32 %0, %14
  %15 = shl nsw i32 %.decomposed, 8
  %16 = udiv i32 %0, 7
  %17 = and i32 %16, 255
  %18 = lshr i32 %0, 28
  %19 = or disjoint i32 %15, %18
  %20 = add i32 %19, %13
  %21 = add i32 %20, %17
  %22 = add i32 %21, %.3
  ret i32 %22
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(argmem: write)
define internal fastcc i32 @count_down(i32 noundef %0, ptr nocapture noundef writeonly %1) unnamed_addr #4 {
  %3 = icmp sgt i32 %0, -4
  br i1 %3, label %.lr.ph, label %._crit_edge

._crit_edge:                                      ; preds = %.lr.ph, %2
  %.08.lcssa = phi i32 [ 0, %2 ], [ %9, %.lr.ph ]
  ret i32 %.08.lcssa

.lr.ph:                                           ; preds = %2, %.lr.ph
  %.010 = phi i32 [ %10, %.lr.ph ], [ %0, %2 ]
  %.089 = phi i32 [ %9, %.lr.ph ], [ 0, %2 ]
  %4 = trunc i32 %.010 to i8
  %5 = add nsw i32 %.010, 3
  %6 = and i32 %5, 15
  %7 = getelementptr inbounds i8, ptr %1, i32 %6
  store i8 %4, ptr %7, align 1, !tbaa !8
  %8 = mul i32 %.089, 31
  %9 = add i32 %8, %.010
  %10 = add nsw i32 %.010, -1
  %11 = icmp sgt i32 %.010, -3
  br i1 %11, label %.lr.ph, label %._crit_edge, !llvm.loop !11
}

attributes #0 = { nofree norecurse nosync nounwind memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #1 = { mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #2 = { nofree noinline norecurse nosync nounwind memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #3 = { mustprogress nofree noinline norecurse nosync nounwind willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #4 = { nofree noinline norecurse nosync nounwind memory(argmem: write) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #5 = { nounwind }

!llvm.module.flags = !{!0, !1, !2}
!llvm.ident = !{!3}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 1, !"target-abi", !"ilp32"}
!2 = !{i32 8, !"SmallDataLimit", i32 8}
!3 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
!4 = !{!5, !5, i64 0}
!5 = !{!"int", !6, i64 0}
!6 = !{!"omnipotent char", !7, i64 0}
!7 = !{!"Simple C/C++ TBAA"}
!8 = !{!6, !6, i64 0}
!9 = distinct !{!9, !10}
!10 = !{!"llvm.loop.mustprogress"}
!11 = distinct !{!11, !10}
