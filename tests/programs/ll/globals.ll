source_filename = "globals.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

%struct.anon = type { i32, ptr }

@counter = dso_local local_unnamed_addr global i32 10, align 4
@.str = private unnamed_addr constant [5 x i8] c"four\00", align 1
@.str.1 = private unnamed_addr constant [5 x i8] c"nine\00", align 1
@pairs = dso_local local_unnamed_addr constant [2 x %struct.anon] [%struct.anon { i32 4, ptr @.str }, %struct.anon { i32 9, ptr @.str.1 }], align 4
@table = internal unnamed_addr constant [8 x i32] [i32 1, i32 1, i32 2, i32 3, i32 5, i32 8, i32 13, i32 21], align 4
@names = internal unnamed_addr constant [4 x ptr] [ptr @.str.2, ptr @.str.3, ptr @.str.4, ptr @.str.5], align 4
@ops = internal unnamed_addr constant [3 x ptr] [ptr @inc, ptr @dbl, ptr @neg], align 4
@.str.2 = private unnamed_addr constant [5 x i8] c"zero\00", align 1
@.str.3 = private unnamed_addr constant [4 x i8] c"one\00", align 1
@.str.4 = private unnamed_addr constant [4 x i8] c"two\00", align 1
@.str.5 = private unnamed_addr constant [6 x i8] c"three\00", align 1

; Function Attrs: nounwind
define dso_local i32 @test(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
  %5 = and i32 %0, 7
  %6 = getelementptr inbounds [8 x i32], ptr @table, i32 0, i32 %5
  %7 = load i32, ptr %6, align 4, !tbaa !4
  %8 = load i32, ptr @counter, align 4, !tbaa !4
  %9 = add nsw i32 %8, %7
  store i32 %9, ptr @counter, align 4, !tbaa !4
  %10 = and i32 %0, 3
  %11 = getelementptr inbounds [4 x ptr], ptr @names, i32 0, i32 %10
  %12 = load ptr, ptr %11, align 4, !tbaa !8
  br label %13

13:                                               ; preds = %13, %4
  %.0.i = phi i32 [ 0, %4 ], [ %16, %13 ]
  %14 = getelementptr inbounds i8, ptr %12, i32 %.0.i
  %15 = load i8, ptr %14, align 1, !tbaa !10
  %.not.i = icmp eq i8 %15, 0
  %16 = add nuw nsw i32 %.0.i, 1
  br i1 %.not.i, label %len.exit, label %13, !llvm.loop !11

len.exit:                                         ; preds = %13
  %17 = srem i32 %1, 3
  %18 = getelementptr inbounds [3 x ptr], ptr @ops, i32 0, i32 %17
  %19 = load ptr, ptr %18, align 4, !tbaa !8
  %20 = tail call i32 %19(i32 noundef %0) #2
  %21 = and i32 %1, 1
  %22 = getelementptr inbounds [2 x %struct.anon], ptr @pairs, i32 0, i32 %21
  %23 = load i32, ptr %22, align 4, !tbaa !13
  %24 = getelementptr inbounds [2 x %struct.anon], ptr @pairs, i32 0, i32 %21, i32 1
  %25 = load ptr, ptr %24, align 4, !tbaa !15
  br label %26

26:                                               ; preds = %26, %len.exit
  %.0.i11 = phi i32 [ 0, %len.exit ], [ %29, %26 ]
  %27 = getelementptr inbounds i8, ptr %25, i32 %.0.i11
  %28 = load i8, ptr %27, align 1, !tbaa !10
  %.not.i12 = icmp eq i8 %28, 0
  %29 = add nuw nsw i32 %.0.i11, 1
  br i1 %.not.i12, label %len.exit13, label %26, !llvm.loop !11

len.exit13:                                       ; preds = %26
  %30 = shl nsw i32 %9, 1
  %31 = add i32 %30, 111
  %32 = add i32 %31, %.0.i
  %33 = add i32 %32, %20
  %34 = add i32 %33, %23
  %35 = add i32 %34, %.0.i11
  ret i32 %35
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none)
define internal i32 @inc(i32 noundef %0) #1 {
  %2 = add nsw i32 %0, 1
  ret i32 %2
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none)
define internal i32 @dbl(i32 noundef %0) #1 {
  %2 = shl nsw i32 %0, 1
  ret i32 %2
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none)
define internal i32 @neg(i32 noundef %0) #1 {
  %2 = sub nsw i32 0, %0
  ret i32 %2
}

attributes #0 = { nounwind "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #1 = { mustprogress nofree norecurse nosync nounwind willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
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
!8 = !{!9, !9, i64 0}
!9 = !{!"any pointer", !6, i64 0}
!10 = !{!6, !6, i64 0}
!11 = distinct !{!11, !12}
!12 = !{!"llvm.loop.mustprogress"}
!13 = !{!14, !5, i64 0}
!14 = !{!"", !5, i64 0, !9, i64 4}
!15 = !{!14, !9, i64 4}
