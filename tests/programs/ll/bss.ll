source_filename = "bss.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

%struct.anon = type { i16, i32, [6 x i8] }

@table = internal unnamed_addr global [1024 x i32] zeroinitializer, align 4
@bytes = internal unnamed_addr global [300 x i8] zeroinitializer, align 1
@wide = internal unnamed_addr global [8 x i64] zeroinitializer, align 8
@mixed = internal unnamed_addr global [4 x %struct.anon] zeroinitializer, align 4

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, argmem: none, inaccessiblemem: none)
define dso_local i32 @test(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
  %5 = and i32 %0, 1023
  %6 = getelementptr inbounds [1024 x i32], ptr @table, i32 0, i32 %5
  %7 = load i32, ptr %6, align 4, !tbaa !4
  %8 = srem i32 %2, 300
  %9 = getelementptr inbounds [300 x i8], ptr @bytes, i32 0, i32 %8
  %10 = load i8, ptr %9, align 1, !tbaa !8
  %11 = zext i8 %10 to i32
  %12 = and i32 %3, 7
  %13 = getelementptr inbounds [8 x i64], ptr @wide, i32 0, i32 %12
  %14 = load i64, ptr %13, align 8, !tbaa !9
  %15 = trunc i64 %14 to i32
  %16 = and i32 %3, 3
  %17 = getelementptr inbounds [4 x %struct.anon], ptr @mixed, i32 0, i32 %16
  %18 = getelementptr inbounds [4 x %struct.anon], ptr @mixed, i32 0, i32 %16, i32 1
  %19 = load i32, ptr %18, align 4, !tbaa !11
  %20 = add nsw i32 %7, %1
  store i32 %20, ptr %6, align 4, !tbaa !4
  %21 = add nsw i32 %1, %0
  %22 = trunc i32 %21 to i8
  store i8 %22, ptr %9, align 1, !tbaa !8
  %23 = sext i32 %0 to i64
  %24 = sext i32 %1 to i64
  %25 = mul nsw i64 %24, %23
  %26 = add nsw i64 %14, %25
  store i64 %26, ptr %13, align 8, !tbaa !9
  %27 = trunc i32 %1 to i16
  store i16 %27, ptr %17, align 4, !tbaa !14
  %28 = trunc i32 %3 to i8
  %29 = getelementptr inbounds [4 x %struct.anon], ptr @mixed, i32 0, i32 %16, i32 2, i32 5
  store i8 %28, ptr %29, align 1, !tbaa !8
  %30 = shl nsw i32 %20, 1
  %31 = and i32 %21, 255
  %32 = lshr i64 %26, 1
  %33 = trunc i64 %32 to i32
  %sext30 = shl i32 %1, 16
  %34 = ashr exact i32 %sext30, 16
  %sext = shl i32 %3, 24
  %35 = ashr exact i32 %sext, 24
  %36 = add nsw i32 %3, 1
  %37 = and i32 %36, 3
  %38 = getelementptr inbounds [4 x %struct.anon], ptr @mixed, i32 0, i32 %37
  %39 = load i16, ptr %38, align 4, !tbaa !14
  %40 = sext i16 %39 to i32
  %41 = add nsw i32 %34, %31
  %42 = add nsw i32 %41, %35
  %43 = add i32 %42, %7
  %44 = add i32 %43, %11
  %45 = add i32 %44, %30
  %46 = add i32 %45, %15
  %47 = add i32 %46, %19
  %48 = add i32 %47, %33
  %49 = add i32 %48, %40
  %50 = load i32, ptr @table, align 4, !tbaa !4
  %51 = add nsw i32 %50, %49
  %52 = load i32, ptr getelementptr inbounds ([1024 x i32], ptr @table, i32 0, i32 97), align 4, !tbaa !4
  %53 = add nsw i32 %52, %51
  %54 = load i32, ptr getelementptr inbounds ([1024 x i32], ptr @table, i32 0, i32 194), align 4, !tbaa !4
  %55 = add nsw i32 %54, %53
  %56 = load i32, ptr getelementptr inbounds ([1024 x i32], ptr @table, i32 0, i32 291), align 4, !tbaa !4
  %57 = add nsw i32 %56, %55
  %58 = load i32, ptr getelementptr inbounds ([1024 x i32], ptr @table, i32 0, i32 388), align 4, !tbaa !4
  %59 = add nsw i32 %58, %57
  %60 = load i32, ptr getelementptr inbounds ([1024 x i32], ptr @table, i32 0, i32 485), align 4, !tbaa !4
  %61 = add nsw i32 %60, %59
  %62 = load i32, ptr getelementptr inbounds ([1024 x i32], ptr @table, i32 0, i32 582), align 4, !tbaa !4
  %63 = add nsw i32 %62, %61
  %64 = load i32, ptr getelementptr inbounds ([1024 x i32], ptr @table, i32 0, i32 679), align 4, !tbaa !4
  %65 = add nsw i32 %64, %63
  %66 = load i32, ptr getelementptr inbounds ([1024 x i32], ptr @table, i32 0, i32 776), align 4, !tbaa !4
  %67 = add nsw i32 %66, %65
  %68 = load i32, ptr getelementptr inbounds ([1024 x i32], ptr @table, i32 0, i32 873), align 4, !tbaa !4
  %69 = add nsw i32 %68, %67
  %70 = load i32, ptr getelementptr inbounds ([1024 x i32], ptr @table, i32 0, i32 970), align 4, !tbaa !4
  %71 = add nsw i32 %70, %69
  ret i32 %71
}

attributes #0 = { mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, argmem: none, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }

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
!9 = !{!10, !10, i64 0}
!10 = !{!"long long", !6, i64 0}
!11 = !{!12, !5, i64 4}
!12 = !{!"", !13, i64 0, !5, i64 4, !6, i64 8}
!13 = !{!"short", !6, i64 0}
!14 = !{!12, !13, i64 0}
