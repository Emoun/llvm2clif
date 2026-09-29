source_filename = "int64.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none)
define dso_local i32 @test(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
  %5 = sext i32 %0 to i64
  %6 = sext i32 %1 to i64
  %7 = mul nsw i64 %6, %5
  %8 = sext i32 %2 to i64
  %9 = add nsw i64 %7, %8
  %10 = zext i32 %0 to i64
  %11 = mul i64 %10, 4294967311
  %12 = zext i32 %1 to i64
  %13 = add i64 %11, %12
  %14 = shl i64 %9, 7
  %15 = ashr exact i64 %14, 3
  %16 = lshr i64 %13, 5
  %17 = add nsw i64 %15, %16
  %18 = lshr i64 %13, 13
  %19 = xor i64 %18, %13
  %20 = mul i64 %19, -7046029254386353131
  %21 = mul i64 %8, 4294967297
  %22 = lshr i64 %21, 13
  %23 = xor i64 %22, %21
  %24 = mul i64 %23, -7046029254386353131
  %25 = xor i64 %24, %20
  %26 = lshr i64 %25, 29
  %27 = xor i64 %20, %26
  %28 = xor i64 %27, %24
  %29 = trunc i64 %9 to i32
  %30 = and i32 %29, 65535
  %31 = lshr i64 %9, 32
  %32 = trunc i64 %31 to i32
  %33 = add nsw i32 %30, %32
  %34 = srem i64 %17, 1000
  %35 = trunc i64 %34 to i32
  %36 = add nsw i32 %33, %35
  %37 = urem i64 %28, 1000
  %38 = trunc i64 %37 to i32
  %39 = add nsw i32 %36, %38
  %40 = icmp slt i64 %9, %14
  %41 = add nsw i32 %39, 10000
  %spec.select = select i1 %40, i32 %41, i32 %39
  %42 = sext i32 %3 to i64
  %43 = icmp ugt i64 %13, %42
  %44 = add nsw i32 %spec.select, 20000
  %.1 = select i1 %43, i32 %44, i32 %spec.select
  %45 = udiv i64 %9, 3
  %46 = urem i64 %45, 100
  %47 = trunc i64 %46 to i32
  %48 = add nsw i32 %1, 5
  %49 = sext i32 %48 to i64
  %50 = sdiv i64 %9, %49
  %51 = srem i64 %50, 100
  %52 = trunc i64 %51 to i32
  %53 = urem i64 %13, 7
  %54 = trunc i64 %53 to i32
  %55 = lshr i64 %28, 60
  %56 = trunc i64 %55 to i32
  %57 = tail call i64 @llvm.ctpop.i64(i64 %13), !range !4
  %58 = trunc i64 %57 to i32
  %59 = or i64 %13, 1
  %60 = tail call i64 @llvm.ctlz.i64(i64 %59, i1 true), !range !5
  %61 = trunc i64 %60 to i32
  %62 = or i64 %13, 68719476736
  %63 = tail call i64 @llvm.cttz.i64(i64 %62, i1 true), !range !6
  %64 = trunc i64 %63 to i32
  %65 = trunc i64 %13 to i32
  %66 = and i32 %65, 255
  %67 = add nuw nsw i32 %54, %58
  %68 = add nuw nsw i32 %67, %66
  %69 = add nuw nsw i32 %68, %61
  %70 = add nuw nsw i32 %69, %64
  %71 = add nuw nsw i32 %70, %47
  %72 = add nuw nsw i32 %71, %56
  %73 = add i32 %72, %.1
  %74 = add i32 %73, %52
  ret i32 %74
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.ctpop.i64(i64) #1

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.ctlz.i64(i64, i1 immarg) #1

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.cttz.i64(i64, i1 immarg) #1

attributes #0 = { mustprogress nofree norecurse nosync nounwind willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #1 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) }

!llvm.module.flags = !{!0, !1, !2}
!llvm.ident = !{!3}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 1, !"target-abi", !"ilp32"}
!2 = !{i32 8, !"SmallDataLimit", i32 8}
!3 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
!4 = !{i64 0, i64 65}
!5 = !{i64 0, i64 64}
!6 = !{i64 0, i64 37}
