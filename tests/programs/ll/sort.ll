source_filename = "sort.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

; Function Attrs: nofree norecurse nosync nounwind memory(none)
define dso_local i32 @test(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
  %5 = alloca [20 x i32], align 4
  call void @llvm.lifetime.start.p0(i64 80, ptr nonnull %5) #2
  %6 = mul i32 %0, -1640531535
  %7 = add i32 %6, 12345
  br label %58

.lr.ph.preheader.i:                               ; preds = %58, %._crit_edge.i
  %indvars.iv.i = phi i32 [ %indvars.iv.next.i, %._crit_edge.i ], [ 19, %58 ]
  %.024.i = phi i32 [ %8, %._crit_edge.i ], [ 0, %58 ]
  %.pre.i = load i32, ptr %5, align 4, !tbaa !4
  br label %.lr.ph.i

._crit_edge.i:                                    ; preds = %56
  %8 = add nuw nsw i32 %.024.i, 1
  %indvars.iv.next.i = add nsw i32 %indvars.iv.i, -1
  %exitcond25.not.i = icmp eq i32 %8, 19
  br i1 %exitcond25.not.i, label %bubble.exit.preheader, label %.lr.ph.preheader.i, !llvm.loop !8

bubble.exit.preheader:                            ; preds = %._crit_edge.i
  %.pre = load i32, ptr %5, align 4, !tbaa !4
  %9 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 1
  %10 = load i32, ptr %9, align 4, !tbaa !4
  %11 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 2
  %12 = load i32, ptr %11, align 4, !tbaa !4
  %13 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 3
  %14 = load i32, ptr %13, align 4, !tbaa !4
  %15 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 4
  %16 = load i32, ptr %15, align 4, !tbaa !4
  %17 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 5
  %18 = load i32, ptr %17, align 4, !tbaa !4
  %19 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 6
  %20 = load i32, ptr %19, align 4, !tbaa !4
  %21 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 7
  %22 = load i32, ptr %21, align 4, !tbaa !4
  %23 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 8
  %24 = load i32, ptr %23, align 4, !tbaa !4
  %25 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 9
  %26 = load i32, ptr %25, align 4, !tbaa !4
  %27 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 10
  %28 = load i32, ptr %27, align 4, !tbaa !4
  %29 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 11
  %30 = load i32, ptr %29, align 4, !tbaa !4
  %31 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 12
  %32 = load i32, ptr %31, align 4, !tbaa !4
  %33 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 13
  %34 = load i32, ptr %33, align 4, !tbaa !4
  %35 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 14
  %36 = load i32, ptr %35, align 4, !tbaa !4
  %37 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 15
  %38 = load i32, ptr %37, align 4, !tbaa !4
  %39 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 16
  %40 = load i32, ptr %39, align 4, !tbaa !4
  %41 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 17
  %42 = load i32, ptr %41, align 4, !tbaa !4
  %43 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 18
  %44 = load i32, ptr %43, align 4, !tbaa !4
  %45 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 19
  %46 = load i32, ptr %45, align 4, !tbaa !4
  %47 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 7
  %48 = load i32, ptr %47, align 4, !tbaa !4
  br label %66

.lr.ph.i:                                         ; preds = %56, %.lr.ph.preheader.i
  %49 = phi i32 [ %57, %56 ], [ %.pre.i, %.lr.ph.preheader.i ]
  %.01923.i = phi i32 [ %50, %56 ], [ 0, %.lr.ph.preheader.i ]
  %50 = add nuw nsw i32 %.01923.i, 1
  %51 = getelementptr inbounds i32, ptr %5, i32 %50
  %52 = load i32, ptr %51, align 4, !tbaa !4
  %53 = icmp sgt i32 %49, %52
  br i1 %53, label %54, label %56

54:                                               ; preds = %.lr.ph.i
  %55 = getelementptr inbounds i32, ptr %5, i32 %.01923.i
  store i32 %52, ptr %55, align 4, !tbaa !4
  store i32 %49, ptr %51, align 4, !tbaa !4
  br label %56

56:                                               ; preds = %54, %.lr.ph.i
  %57 = phi i32 [ %52, %.lr.ph.i ], [ %49, %54 ]
  %exitcond.not.i = icmp eq i32 %50, %indvars.iv.i
  br i1 %exitcond.not.i, label %._crit_edge.i, label %.lr.ph.i, !llvm.loop !10

58:                                               ; preds = %4, %58
  %.01637 = phi i32 [ 0, %4 ], [ %65, %58 ]
  %.01736 = phi i32 [ %7, %4 ], [ %60, %58 ]
  %59 = mul i32 %.01736, 1103515245
  %60 = add i32 %59, 12345
  %61 = lshr i32 %60, 16
  %.lhs.trunc = trunc i32 %61 to i16
  %62 = urem i16 %.lhs.trunc, 100
  %.zext = zext nneg i16 %62 to i32
  %63 = add nsw i32 %.zext, %1
  %64 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 %.01637
  store i32 %63, ptr %64, align 4, !tbaa !4
  %65 = add nuw nsw i32 %.01637, 1
  %exitcond.not = icmp eq i32 %65, 20
  br i1 %exitcond.not, label %.lr.ph.preheader.i, label %58, !llvm.loop !11

66:                                               ; preds = %72, %bubble.exit.preheader
  %.01730.i = phi i32 [ 19, %bubble.exit.preheader ], [ %.118.i, %72 ]
  %.02029.i = phi i32 [ 0, %bubble.exit.preheader ], [ %.121.i, %72 ]
  %67 = sub nsw i32 %.01730.i, %.02029.i
  %68 = sdiv i32 %67, 2
  %69 = add nsw i32 %68, %.02029.i
  %70 = getelementptr inbounds i32, ptr %5, i32 %69
  %71 = load i32, ptr %70, align 4, !tbaa !4
  %.not25.i = icmp eq i32 %71, %48
  br i1 %.not25.i, label %bsearch_int.exit, label %72

72:                                               ; preds = %66
  %73 = icmp slt i32 %71, %48
  %74 = add nsw i32 %69, 1
  %75 = add nsw i32 %69, -1
  %.121.i = select i1 %73, i32 %74, i32 %.02029.i
  %.118.i = select i1 %73, i32 %.01730.i, i32 %75
  %.not.i = icmp sgt i32 %.121.i, %.118.i
  br i1 %.not.i, label %bsearch_int.exit.thread.preheader, label %66

bsearch_int.exit:                                 ; preds = %66
  %76 = icmp eq i32 %69, 7
  br i1 %76, label %90, label %bsearch_int.exit.thread.preheader

bsearch_int.exit.thread.preheader:                ; preds = %72, %bsearch_int.exit
  br label %bsearch_int.exit.thread

bsearch_int.exit.thread:                          ; preds = %bsearch_int.exit.thread.preheader, %82
  %.01730.i18 = phi i32 [ %.118.i22, %82 ], [ 19, %bsearch_int.exit.thread.preheader ]
  %.02029.i19 = phi i32 [ %.121.i21, %82 ], [ 0, %bsearch_int.exit.thread.preheader ]
  %77 = sub nsw i32 %.01730.i18, %.02029.i19
  %78 = sdiv i32 %77, 2
  %79 = add nsw i32 %78, %.02029.i19
  %80 = getelementptr inbounds i32, ptr %5, i32 %79
  %81 = load i32, ptr %80, align 4, !tbaa !4
  %.not25.i20 = icmp eq i32 %81, %48
  br i1 %.not25.i20, label %bsearch_int.exit25, label %82

82:                                               ; preds = %bsearch_int.exit.thread
  %83 = icmp slt i32 %81, %48
  %84 = add nsw i32 %79, 1
  %85 = add nsw i32 %79, -1
  %.121.i21 = select i1 %83, i32 %84, i32 %.02029.i19
  %.118.i22 = select i1 %83, i32 %.01730.i18, i32 %85
  %.not.i23 = icmp sgt i32 %.121.i21, %.118.i22
  br i1 %.not.i23, label %bsearch_int.exit25, label %bsearch_int.exit.thread

bsearch_int.exit25:                               ; preds = %bsearch_int.exit.thread, %82
  %.2.i24 = phi i32 [ -1, %82 ], [ %79, %bsearch_int.exit.thread ]
  %86 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 %.2.i24
  %87 = load i32, ptr %86, align 4, !tbaa !4
  %88 = icmp eq i32 %87, %48
  %89 = zext i1 %88 to i32
  br label %90

90:                                               ; preds = %bsearch_int.exit25, %bsearch_int.exit
  %91 = phi i32 [ 1, %bsearch_int.exit ], [ %89, %bsearch_int.exit25 ]
  br label %92

92:                                               ; preds = %98, %90
  %.01730.i26 = phi i32 [ 19, %90 ], [ %.118.i30, %98 ]
  %.02029.i27 = phi i32 [ 0, %90 ], [ %.121.i29, %98 ]
  %93 = sub nsw i32 %.01730.i26, %.02029.i27
  %94 = sdiv i32 %93, 2
  %95 = add nsw i32 %94, %.02029.i27
  %96 = getelementptr inbounds i32, ptr %5, i32 %95
  %97 = load i32, ptr %96, align 4, !tbaa !4
  %.not25.i28 = icmp eq i32 %97, 1000
  br i1 %.not25.i28, label %bsearch_int.exit33, label %98

98:                                               ; preds = %92
  %99 = icmp slt i32 %97, 1000
  %100 = add nsw i32 %95, 1
  %101 = add nsw i32 %95, -1
  %.121.i29 = select i1 %99, i32 %100, i32 %.02029.i27
  %.118.i30 = select i1 %99, i32 %.01730.i26, i32 %101
  %.not.i31 = icmp sgt i32 %.121.i29, %.118.i30
  br i1 %.not.i31, label %bsearch_int.exit33, label %92

bsearch_int.exit33:                               ; preds = %92, %98
  %.2.i32 = phi i32 [ -1, %98 ], [ %95, %92 ]
  %102 = icmp sle i32 %.pre, %10
  %103 = zext i1 %102 to i32
  %104 = icmp sle i32 %10, %12
  %105 = zext i1 %104 to i32
  %106 = add nuw nsw i32 %103, %105
  %107 = icmp sle i32 %12, %14
  %108 = zext i1 %107 to i32
  %109 = add nuw nsw i32 %106, %108
  %110 = icmp sle i32 %14, %16
  %111 = zext i1 %110 to i32
  %112 = add nuw nsw i32 %109, %111
  %113 = icmp sle i32 %16, %18
  %114 = zext i1 %113 to i32
  %115 = add nuw nsw i32 %112, %114
  %116 = icmp sle i32 %18, %20
  %117 = zext i1 %116 to i32
  %118 = add nuw nsw i32 %115, %117
  %119 = icmp sle i32 %20, %22
  %120 = zext i1 %119 to i32
  %121 = add nuw nsw i32 %118, %120
  %122 = icmp sle i32 %22, %24
  %123 = zext i1 %122 to i32
  %124 = add nuw nsw i32 %121, %123
  %125 = icmp sle i32 %24, %26
  %126 = zext i1 %125 to i32
  %127 = add nuw nsw i32 %124, %126
  %128 = icmp sle i32 %26, %28
  %129 = zext i1 %128 to i32
  %130 = add nuw nsw i32 %127, %129
  %131 = icmp sle i32 %28, %30
  %132 = zext i1 %131 to i32
  %133 = add nuw nsw i32 %130, %132
  %134 = icmp sle i32 %30, %32
  %135 = zext i1 %134 to i32
  %136 = add nuw nsw i32 %133, %135
  %137 = icmp sle i32 %32, %34
  %138 = zext i1 %137 to i32
  %139 = add nuw nsw i32 %136, %138
  %140 = icmp sle i32 %34, %36
  %141 = zext i1 %140 to i32
  %142 = add nuw nsw i32 %139, %141
  %143 = icmp sle i32 %36, %38
  %144 = zext i1 %143 to i32
  %145 = add nuw nsw i32 %142, %144
  %146 = icmp sle i32 %38, %40
  %147 = zext i1 %146 to i32
  %148 = add nuw nsw i32 %145, %147
  %149 = icmp sle i32 %40, %42
  %150 = zext i1 %149 to i32
  %151 = add nuw nsw i32 %148, %150
  %152 = icmp sle i32 %42, %44
  %153 = zext i1 %152 to i32
  %154 = add nuw nsw i32 %151, %153
  %155 = icmp sle i32 %44, %46
  %156 = zext i1 %155 to i32
  %157 = add nuw nsw i32 %154, %156
  %158 = add nuw nsw i32 %91, %157
  %159 = icmp eq i32 %.2.i32, -1
  %160 = zext i1 %159 to i32
  %161 = add nuw nsw i32 %158, %160
  %162 = mul nsw i32 %161, 1000
  %163 = getelementptr inbounds [20 x i32], ptr %5, i32 0, i32 19
  %164 = load i32, ptr %163, align 4, !tbaa !4
  %165 = add i32 %164, %.pre
  %166 = add i32 %165, %162
  call void @llvm.lifetime.end.p0(i64 80, ptr nonnull %5) #2
  ret i32 %166
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #1

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #1

attributes #0 = { nofree norecurse nosync nounwind memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #1 = { mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #2 = { nounwind }

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
!8 = distinct !{!8, !9}
!9 = !{!"llvm.loop.mustprogress"}
!10 = distinct !{!10, !9}
!11 = distinct !{!11, !9}
