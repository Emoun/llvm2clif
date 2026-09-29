source_filename = "malloc.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

%struct.Node = type { i32, ptr }

; Function Attrs: nofree nounwind memory(readwrite, argmem: read)
define dso_local i32 @test(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
  %5 = icmp sgt i32 %0, 0
  br i1 %5, label %.lr.ph, label %._crit_edge

.lr.ph:                                           ; preds = %4, %.lr.ph
  %.02633 = phi ptr [ %8, %.lr.ph ], [ null, %4 ]
  %.03032 = phi i32 [ %10, %.lr.ph ], [ 0, %4 ]
  %6 = mul nsw i32 %.03032, 3
  %7 = add nsw i32 %6, %1
  %8 = tail call noalias noundef dereferenceable_or_null(8) ptr @malloc(i32 noundef 8) #2
  store i32 %7, ptr %8, align 4, !tbaa !4
  %9 = getelementptr inbounds %struct.Node, ptr %8, i32 0, i32 1
  store ptr %.02633, ptr %9, align 4, !tbaa !10
  %10 = add nuw nsw i32 %.03032, 1
  %exitcond.not = icmp eq i32 %10, %0
  br i1 %exitcond.not, label %.lr.ph38, label %.lr.ph, !llvm.loop !11

._crit_edge.loopexit:                             ; preds = %.lr.ph38
  %11 = mul nsw i32 %34, 100
  br label %._crit_edge

._crit_edge:                                      ; preds = %4, %._crit_edge.loopexit
  %.029.lcssa = phi i32 [ %33, %._crit_edge.loopexit ], [ 0, %4 ]
  %.028.lcssa = phi i32 [ %11, %._crit_edge.loopexit ], [ 0, %4 ]
  %12 = add nsw i32 %1, 1
  %13 = add nsw i32 %1, 2
  %14 = add nsw i32 %1, 3
  %15 = add nsw i32 %1, 4
  %16 = add nsw i32 %1, 5
  %17 = add nsw i32 %1, 6
  %18 = add nsw i32 %1, 7
  %19 = add nsw i32 %1, 8
  %20 = add nsw i32 %1, 9
  %21 = add nsw i32 %.029.lcssa, %1
  %22 = add nsw i32 %12, %21
  %23 = add nsw i32 %13, %22
  %24 = add nsw i32 %14, %23
  %25 = add nsw i32 %15, %24
  %26 = add nsw i32 %16, %25
  %27 = add nsw i32 %17, %26
  %28 = add nsw i32 %18, %27
  %29 = add nsw i32 %19, %28
  %30 = add nsw i32 %20, %29
  %31 = add nsw i32 %30, %.028.lcssa
  ret i32 %31

.lr.ph38:                                         ; preds = %.lr.ph, %.lr.ph38
  %.02737 = phi ptr [ %36, %.lr.ph38 ], [ %8, %.lr.ph ]
  %.02836 = phi i32 [ %34, %.lr.ph38 ], [ 0, %.lr.ph ]
  %.02935 = phi i32 [ %33, %.lr.ph38 ], [ 0, %.lr.ph ]
  %32 = load i32, ptr %.02737, align 4, !tbaa !4
  %33 = add nsw i32 %32, %.02935
  %34 = add nuw nsw i32 %.02836, 1
  %35 = getelementptr inbounds %struct.Node, ptr %.02737, i32 0, i32 1
  %36 = load ptr, ptr %35, align 4, !tbaa !10
  %.not = icmp eq ptr %36, null
  br i1 %.not, label %._crit_edge.loopexit, label %.lr.ph38, !llvm.loop !13
}

; Function Attrs: mustprogress nofree nounwind willreturn allockind("alloc,uninitialized") allocsize(0) memory(inaccessiblemem: readwrite)
declare dso_local noalias noundef ptr @malloc(i32 noundef) local_unnamed_addr #1

attributes #0 = { nofree nounwind memory(readwrite, argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #1 = { mustprogress nofree nounwind willreturn allockind("alloc,uninitialized") allocsize(0) memory(inaccessiblemem: readwrite) "alloc-family"="malloc" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #2 = { allocsize(0) }

!llvm.module.flags = !{!0, !1, !2}
!llvm.ident = !{!3}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 1, !"target-abi", !"ilp32"}
!2 = !{i32 8, !"SmallDataLimit", i32 8}
!3 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
!4 = !{!5, !6, i64 0}
!5 = !{!"Node", !6, i64 0, !9, i64 4}
!6 = !{!"int", !7, i64 0}
!7 = !{!"omnipotent char", !8, i64 0}
!8 = !{!"Simple C/C++ TBAA"}
!9 = !{!"any pointer", !7, i64 0}
!10 = !{!5, !9, i64 4}
!11 = distinct !{!11, !12}
!12 = !{!"llvm.loop.mustprogress"}
!13 = distinct !{!13, !12}
