source_filename = "chars.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

@.str = private unnamed_addr constant [12 x i8] c"hello, scry\00", align 1
@.str.1 = private unnamed_addr constant [12 x i8] c"HELLO, SCRY\00", align 1

; Function Attrs: nofree nosync nounwind memory(none)
define dso_local i32 @test(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
.lr.ph.i.preheader:
  %4 = alloca [32 x i8], align 1
  call void @llvm.lifetime.start.p0(i64 32, ptr nonnull %4) #5
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 1 dereferenceable(12) %4, ptr noundef nonnull align 1 dereferenceable(12) @.str, i32 12, i1 false)
  br label %.lr.ph.i

.lr.ph.i:                                         ; preds = %.lr.ph.i.preheader, %8
  %5 = phi i8 [ %10, %8 ], [ 104, %.lr.ph.i.preheader ]
  %.08.i = phi ptr [ %9, %8 ], [ %4, %.lr.ph.i.preheader ]
  %6 = add i8 %5, -97
  %or.cond.i = icmp ult i8 %6, 26
  br i1 %or.cond.i, label %7, label %8

7:                                                ; preds = %.lr.ph.i
  %narrow.i = add nsw i8 %5, -32
  store i8 %narrow.i, ptr %.08.i, align 1, !tbaa !4
  br label %8

8:                                                ; preds = %7, %.lr.ph.i
  %9 = getelementptr inbounds i8, ptr %.08.i, i32 1
  %10 = load i8, ptr %9, align 1, !tbaa !4
  %.not.i = icmp eq i8 %10, 0
  br i1 %.not.i, label %upper.exit, label %.lr.ph.i, !llvm.loop !7

upper.exit:                                       ; preds = %8
  %11 = trunc i32 %0 to i8
  store i8 %11, ptr %4, align 1, !tbaa !4
  %.not5.i = icmp eq i8 %11, 0
  br i1 %.not5.i, label %hash.exit, label %.lr.ph.i18

.lr.ph.i18:                                       ; preds = %upper.exit, %.lr.ph.i18
  %12 = phi i8 [ %17, %.lr.ph.i18 ], [ %11, %upper.exit ]
  %.07.i = phi i32 [ %16, %.lr.ph.i18 ], [ 5381, %upper.exit ]
  %.036.i = phi ptr [ %14, %.lr.ph.i18 ], [ %4, %upper.exit ]
  %13 = mul i32 %.07.i, 33
  %14 = getelementptr inbounds i8, ptr %.036.i, i32 1
  %15 = zext i8 %12 to i32
  %16 = add i32 %13, %15
  %17 = load i8, ptr %14, align 1, !tbaa !4
  %.not.i19 = icmp eq i8 %17, 0
  br i1 %.not.i19, label %hash.exit.loopexit, label %.lr.ph.i18, !llvm.loop !9

hash.exit.loopexit:                               ; preds = %.lr.ph.i18
  %18 = and i32 %16, 65535
  br label %hash.exit

hash.exit:                                        ; preds = %hash.exit.loopexit, %upper.exit
  %.0.lcssa.i = phi i32 [ 5381, %upper.exit ], [ %18, %hash.exit.loopexit ]
  %sext = shl i32 %1, 24
  %19 = ashr exact i32 %sext, 24
  %20 = and i32 %1, 255
  %21 = shl nuw nsw i32 %20, 1
  %.lobit = lshr i32 %19, 31
  %22 = icmp ugt i32 %20, 100
  %23 = zext i1 %22 to i32
  %sext20 = shl i32 %0, 16
  %24 = ashr exact i32 %sext20, 16
  %sext21 = shl i32 %1, 16
  %25 = ashr exact i32 %sext21, 16
  %memcmp = call i32 @memcmp(ptr noundef nonnull dereferenceable(12) %4, ptr noundef nonnull dereferenceable(12) @.str.1, i32 12)
  %26 = icmp eq i32 %memcmp, 0
  %27 = zext i1 %26 to i32
  %28 = call i32 @strlen(ptr noundef nonnull dereferenceable(1) %4)
  %29 = call ptr @strchr(ptr noundef nonnull dereferenceable(1) %4, i32 noundef 83)
  %.not = icmp ne ptr %29, null
  %30 = zext i1 %.not to i32
  %31 = add nsw i32 %24, 132
  %32 = add nsw i32 %31, %25
  %33 = add nsw i32 %32, %21
  %34 = add nsw i32 %33, %19
  %35 = add nsw i32 %34, %.lobit
  %36 = add nsw i32 %35, %23
  %37 = add nsw i32 %36, %.0.lcssa.i
  %38 = add i32 %37, %28
  %39 = add i32 %38, %27
  %40 = add i32 %39, %30
  call void @llvm.lifetime.end.p0(i64 32, ptr nonnull %4) #5
  ret i32 %40
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #1

; Function Attrs: mustprogress nofree nounwind willreturn memory(argmem: read)
declare dso_local i32 @strlen(ptr nocapture noundef) local_unnamed_addr #2

; Function Attrs: mustprogress nofree nounwind willreturn memory(argmem: read)
declare dso_local ptr @strchr(ptr noundef, i32 noundef) local_unnamed_addr #2

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #1

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i32(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i32, i1 immarg) #3

; Function Attrs: nofree nounwind willreturn memory(argmem: read)
declare i32 @memcmp(ptr nocapture, ptr nocapture, i32) local_unnamed_addr #4

attributes #0 = { nofree nosync nounwind memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #1 = { mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #2 = { mustprogress nofree nounwind willreturn memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #3 = { nocallback nofree nounwind willreturn memory(argmem: readwrite) }
attributes #4 = { nofree nounwind willreturn memory(argmem: read) }
attributes #5 = { nounwind }

!llvm.module.flags = !{!0, !1, !2}
!llvm.ident = !{!3}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 1, !"target-abi", !"ilp32"}
!2 = !{i32 8, !"SmallDataLimit", i32 8}
!3 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
!4 = !{!5, !5, i64 0}
!5 = !{!"omnipotent char", !6, i64 0}
!6 = !{!"Simple C/C++ TBAA"}
!7 = distinct !{!7, !8}
!8 = !{!"llvm.loop.mustprogress"}
!9 = distinct !{!9, !8}
