source_filename = "memops.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

%struct.Rec = type { i32, [12 x i8], [8 x i32] }

@.str = private unnamed_addr constant [7 x i8] c"record\00", align 1

; Function Attrs: nofree nosync nounwind memory(none)
define dso_local i32 @test(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
  %5 = alloca %struct.Rec, align 4
  %6 = alloca %struct.Rec, align 4
  %7 = alloca [40 x i32], align 4
  %8 = alloca [300 x i8], align 1
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %5) #5
  %9 = getelementptr inbounds i8, ptr %5, i32 8
  store i64 0, ptr %9, align 4
  store i32 %0, ptr %5, align 4, !tbaa !4
  %10 = getelementptr inbounds %struct.Rec, ptr %5, i32 0, i32 2, i32 0
  store i32 %1, ptr %10, align 4, !tbaa !9
  %11 = add nsw i32 %0, %1
  %12 = getelementptr inbounds %struct.Rec, ptr %5, i32 0, i32 2, i32 1
  store i32 %11, ptr %12, align 4, !tbaa !9
  %13 = shl nsw i32 %0, 1
  %14 = add nsw i32 %13, %1
  %15 = getelementptr inbounds %struct.Rec, ptr %5, i32 0, i32 2, i32 2
  store i32 %14, ptr %15, align 4, !tbaa !9
  %16 = mul nsw i32 %0, 3
  %17 = add nsw i32 %16, %1
  %18 = getelementptr inbounds %struct.Rec, ptr %5, i32 0, i32 2, i32 3
  store i32 %17, ptr %18, align 4, !tbaa !9
  %19 = shl nsw i32 %0, 2
  %20 = add nsw i32 %19, %1
  %21 = getelementptr inbounds %struct.Rec, ptr %5, i32 0, i32 2, i32 4
  store i32 %20, ptr %21, align 4, !tbaa !9
  %22 = mul nsw i32 %0, 5
  %23 = add nsw i32 %22, %1
  %24 = getelementptr inbounds %struct.Rec, ptr %5, i32 0, i32 2, i32 5
  store i32 %23, ptr %24, align 4, !tbaa !9
  %25 = mul nsw i32 %0, 6
  %26 = add nsw i32 %25, %1
  %27 = getelementptr inbounds %struct.Rec, ptr %5, i32 0, i32 2, i32 6
  store i32 %26, ptr %27, align 4, !tbaa !9
  %28 = mul nsw i32 %0, 7
  %29 = add nsw i32 %28, %1
  %30 = getelementptr inbounds %struct.Rec, ptr %5, i32 0, i32 2, i32 7
  store i32 %29, ptr %30, align 4, !tbaa !9
  %31 = getelementptr inbounds %struct.Rec, ptr %5, i32 0, i32 1
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(7) %31, ptr noundef nonnull align 1 dereferenceable(7) @.str, i32 7, i1 false)
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %6) #5
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(48) %6, ptr noundef nonnull align 4 dereferenceable(48) %5, i32 48, i1 false), !tbaa.struct !10
  %32 = getelementptr inbounds %struct.Rec, ptr %6, i32 0, i32 2, i32 3
  %33 = load i32, ptr %32, align 4, !tbaa !9
  %34 = add nsw i32 %33, 7
  store i32 %34, ptr %32, align 4, !tbaa !9
  call void @llvm.lifetime.start.p0(i64 160, ptr nonnull %7) #5
  br label %39

35:                                               ; preds = %39
  %36 = getelementptr inbounds i32, ptr %7, i32 5
  call void @llvm.memmove.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(80) %36, ptr noundef nonnull align 4 dereferenceable(80) %7, i32 80, i1 false)
  %37 = getelementptr inbounds i32, ptr %7, i32 10
  call void @llvm.memmove.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(100) %7, ptr noundef nonnull align 4 dereferenceable(100) %37, i32 100, i1 false)
  call void @llvm.lifetime.start.p0(i64 300, ptr nonnull %8) #5
  %38 = trunc i32 %1 to i8
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 1 dereferenceable(300) %8, i8 %38, i32 300, i1 false)
  br label %43

39:                                               ; preds = %4, %39
  %.02729 = phi i32 [ 0, %4 ], [ %42, %39 ]
  %40 = add nsw i32 %.02729, %0
  %41 = getelementptr inbounds [40 x i32], ptr %7, i32 0, i32 %.02729
  store i32 %40, ptr %41, align 4, !tbaa !9
  %42 = add nuw nsw i32 %.02729, 1
  %exitcond.not = icmp eq i32 %42, 40
  br i1 %exitcond.not, label %35, label %39, !llvm.loop !12

43:                                               ; preds = %35, %43
  %.02431 = phi i32 [ 0, %35 ], [ %48, %43 ]
  %.02530 = phi i32 [ 0, %35 ], [ %47, %43 ]
  %44 = getelementptr inbounds [300 x i8], ptr %8, i32 0, i32 %.02431
  %45 = load i8, ptr %44, align 1, !tbaa !11
  %46 = zext i8 %45 to i32
  %47 = add nuw nsw i32 %.02530, %46
  %48 = add nuw nsw i32 %.02431, 1
  %exitcond35.not = icmp eq i32 %48, 300
  br i1 %exitcond35.not, label %.preheader, label %43, !llvm.loop !14

49:                                               ; preds = %.preheader
  %50 = getelementptr inbounds %struct.Rec, ptr %6, i32 0, i32 1
  %51 = call i32 @strlen(ptr noundef nonnull dereferenceable(1) %50)
  %52 = call i32 @memcmp(ptr noundef nonnull dereferenceable(48) %5, ptr noundef nonnull dereferenceable(48) %6, i32 noundef 48)
  %53 = icmp ne i32 %52, 0
  %54 = zext i1 %53 to i32
  %55 = add i32 %0, 100
  %56 = add i32 %55, %34
  %57 = add i32 %56, %63
  %58 = sub i32 %57, %17
  %59 = add i32 %58, %51
  %60 = add nsw i32 %59, %54
  call void @llvm.lifetime.end.p0(i64 300, ptr nonnull %8) #5
  call void @llvm.lifetime.end.p0(i64 160, ptr nonnull %7) #5
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %6) #5
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %5) #5
  ret i32 %60

.preheader:                                       ; preds = %43, %.preheader
  %.033 = phi i32 [ %64, %.preheader ], [ 0, %43 ]
  %.132 = phi i32 [ %63, %.preheader ], [ %47, %43 ]
  %61 = getelementptr inbounds [40 x i32], ptr %7, i32 0, i32 %.033
  %62 = load i32, ptr %61, align 4, !tbaa !9
  %63 = add nsw i32 %62, %.132
  %64 = add nuw nsw i32 %.033, 1
  %exitcond36.not = icmp eq i32 %64, 40
  br i1 %exitcond36.not, label %49, label %.preheader, !llvm.loop !15
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #1

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i32(ptr nocapture writeonly, i8, i32, i1 immarg) #2

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #1

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i32(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i32, i1 immarg) #3

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memmove.p0.p0.i32(ptr nocapture writeonly, ptr nocapture readonly, i32, i1 immarg) #3

; Function Attrs: mustprogress nofree nounwind willreturn memory(argmem: read)
declare dso_local i32 @strlen(ptr nocapture noundef) local_unnamed_addr #4

; Function Attrs: mustprogress nofree nounwind willreturn memory(argmem: read)
declare dso_local i32 @memcmp(ptr nocapture noundef, ptr nocapture noundef, i32 noundef) local_unnamed_addr #4

attributes #0 = { nofree nosync nounwind memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #1 = { mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #2 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #3 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite) }
attributes #4 = { mustprogress nofree nounwind willreturn memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #5 = { nounwind }

!llvm.module.flags = !{!0, !1, !2}
!llvm.ident = !{!3}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 1, !"target-abi", !"ilp32"}
!2 = !{i32 8, !"SmallDataLimit", i32 8}
!3 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
!4 = !{!5, !6, i64 0}
!5 = !{!"Rec", !6, i64 0, !7, i64 4, !7, i64 16}
!6 = !{!"int", !7, i64 0}
!7 = !{!"omnipotent char", !8, i64 0}
!8 = !{!"Simple C/C++ TBAA"}
!9 = !{!6, !6, i64 0}
!10 = !{i64 0, i64 4, !9, i64 4, i64 12, !11, i64 16, i64 32, !11}
!11 = !{!7, !7, i64 0}
!12 = distinct !{!12, !13}
!13 = !{!"llvm.loop.mustprogress"}
!14 = distinct !{!14, !13}
!15 = distinct !{!15, !13}
