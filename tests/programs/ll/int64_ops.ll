source_filename = "int64_ops.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

%struct.pair = type { i32, i64, i64 }

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none)
define dso_local i32 @test(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
  %5 = alloca i64, align 8
  %6 = alloca %struct.pair, align 8
  %7 = and i32 %2, 63
  %8 = and i32 %3, 63
  %9 = zext i32 %0 to i64
  %10 = shl nuw i64 %9, 32
  %11 = zext i32 %1 to i64
  %12 = or disjoint i64 %10, %11
  %13 = zext nneg i32 %7 to i64
  %14 = shl i64 %11, %13
  %15 = trunc i64 %14 to i32
  %16 = lshr i32 %15, 7
  %17 = and i32 %16, 255
  %18 = zext nneg i32 %8 to i64
  %19 = lshr i64 %12, %18
  %20 = trunc i64 %19 to i32
  %21 = and i32 %20, 255
  %22 = ashr i64 %12, %13
  %23 = trunc i64 %22 to i32
  %24 = and i32 %23, 255
  %25 = ashr i64 %12, %18
  %26 = lshr i64 %25, 32
  %27 = trunc i64 %26 to i32
  %28 = and i32 %27, 255
  %29 = lshr i32 %1, 8
  %30 = and i32 %29, 255
  %31 = lshr i32 %0, 1
  %32 = and i32 %31, 255
  %33 = lshr i64 %12, 31
  %34 = trunc i64 %33 to i32
  %35 = and i32 %34, 255
  %36 = shl i32 %0, 1
  %reass.add = and i32 %36, 510
  %.mask = and i32 %1, 32768
  %isneg.not = icmp eq i32 %.mask, 0
  %37 = select i1 %isneg.not, i32 0, i32 255
  %38 = lshr i32 %1, 3
  %39 = and i32 %38, 255
  %40 = zext i32 %2 to i64
  %41 = lshr i32 %2, 2
  %42 = and i32 %2, 31
  %43 = zext nneg i32 %42 to i64
  %44 = tail call fastcc i64 @sum_squares(i64 noundef %43)
  %45 = trunc i64 %44 to i32
  %46 = and i32 %45, 65535
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5) #6
  %47 = xor i64 %12, 19088743
  %48 = call fastcc i64 @mul_parts(i64 noundef %12, i64 noundef %47, ptr noundef nonnull %5), !range !4
  %49 = trunc i64 %48 to i32
  %50 = and i32 %49, 255
  %51 = load i64, ptr %5, align 8, !tbaa !5
  %52 = lshr i64 %51, 60
  %53 = trunc i64 %52 to i32
  %54 = tail call fastcc i64 @rotl64(i64 noundef %12, i32 noundef %7)
  %55 = trunc i64 %54 to i32
  %56 = and i32 %55, 255
  %57 = tail call fastcc i64 @rotl64(i64 noundef %12, i32 noundef 13)
  %58 = lshr i64 %57, 56
  %59 = trunc i64 %58 to i32
  %60 = sext i32 %1 to i64
  %61 = icmp slt i64 %12, %60
  %62 = zext i1 %61 to i32
  %63 = add nuw nsw i32 %reass.add, %32
  %64 = add nuw nsw i32 %63, %30
  %65 = add nuw nsw i32 %64, %39
  %66 = add nuw nsw i32 %65, %41
  %67 = add nuw nsw i32 %66, %37
  %68 = add nuw nsw i32 %67, %62
  %69 = add nuw nsw i32 %68, %35
  %70 = add nuw nsw i32 %69, %24
  %71 = add nuw nsw i32 %70, %21
  %72 = add nuw nsw i32 %71, %17
  %73 = add nuw nsw i32 %72, %28
  %74 = add nuw nsw i32 %73, %46
  %75 = add nuw nsw i32 %74, %50
  %76 = add nuw nsw i32 %75, %53
  %77 = add nuw nsw i32 %76, %56
  %spec.select = add nuw nsw i32 %77, %59
  %78 = sext i32 %2 to i64
  %79 = sub nsw i64 0, %78
  %.not = icmp slt i64 %12, %79
  %80 = add nuw nsw i32 %spec.select, 2
  %.1 = select i1 %.not, i32 %spec.select, i32 %80
  %81 = sext i32 %3 to i64
  %82 = icmp ugt i64 %12, %81
  %83 = add nuw nsw i32 %.1, 4
  %.2 = select i1 %82, i32 %83, i32 %.1
  %84 = icmp ult i64 %12, 4294967296
  %85 = add nuw nsw i32 %.2, 8
  %.3 = select i1 %84, i32 %85, i32 %.2
  %86 = sext i32 %0 to i64
  %87 = mul nsw i64 %86, 1000000007
  %88 = mul nsw i64 %60, 1000000009
  %89 = icmp sgt i64 %87, %88
  %90 = add nuw nsw i32 %.3, 16
  %.4 = select i1 %89, i32 %90, i32 %.3
  %91 = tail call i64 @llvm.smin.i64(i64 %12, i64 %86)
  %92 = trunc i64 %91 to i32
  %93 = and i32 %92, 255
  %94 = lshr i32 %0, 8
  %95 = mul nsw i64 %86, 3
  %96 = tail call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %12, i64 %95)
  %97 = extractvalue { i64, i1 } %96, 1
  %98 = extractvalue { i64, i1 } %96, 0
  %99 = trunc i64 %98 to i32
  %100 = and i32 %99, 255
  %.5.v = select i1 %97, i32 32, i32 %100
  %101 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %12, i64 %40)
  %102 = extractvalue { i64, i1 } %101, 1
  %103 = extractvalue { i64, i1 } %101, 0
  %104 = lshr i64 %103, 56
  %105 = trunc i64 %104 to i32
  %.6.v = select i1 %102, i32 64, i32 %105
  %106 = tail call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %12, i64 %60)
  %107 = extractvalue { i64, i1 } %106, 1
  %108 = extractvalue { i64, i1 } %106, 0
  %109 = trunc i64 %108 to i32
  %110 = and i32 %109, 255
  %.7.v = select i1 %107, i32 128, i32 %110
  %111 = zext i32 %3 to i64
  %112 = tail call { i64, i1 } @llvm.usub.with.overflow.i64(i64 %12, i64 %111)
  %113 = extractvalue { i64, i1 } %112, 1
  %114 = extractvalue { i64, i1 } %112, 0
  %115 = trunc i64 %114 to i32
  %116 = and i32 %115, 255
  %.8.v = select i1 %113, i32 256, i32 %116
  %117 = or i64 %12, 1
  %118 = lshr i64 %12, 20
  %119 = or i64 %118, 1
  %.frozen = freeze i64 %12
  %.frozen121 = freeze i64 %119
  %120 = udiv i64 %.frozen, %.frozen121
  %121 = trunc i64 %120 to i32
  %122 = and i32 %121, 255
  %123 = mul i64 %120, %.frozen121
  %.decomposed = sub i64 %.frozen, %123
  %124 = trunc i64 %.decomposed to i32
  %125 = and i32 %124, 255
  %126 = or i32 %2, 1
  %127 = sext i32 %126 to i64
  %.frozen122 = freeze i64 %12
  %128 = sdiv i64 %.frozen122, %127
  %129 = trunc i64 %128 to i32
  %130 = and i32 %129, 255
  %131 = mul i64 %128, %127
  %.decomposed123 = sub i64 %.frozen122, %131
  %132 = trunc i64 %.decomposed123 to i32
  %133 = and i32 %132, 255
  %134 = mul nsw i64 %86, -7
  %135 = add nsw i64 %134, -3
  %136 = and i32 %1, 65534
  %137 = or disjoint i32 %136, 1
  %138 = zext nneg i32 %137 to i64
  %139 = sdiv i64 %135, %138
  %140 = trunc i64 %139 to i32
  %141 = and i32 %140, 255
  %142 = or i64 %12, 3
  %143 = udiv i64 -1, %142
  %144 = trunc i64 %143 to i32
  %145 = and i32 %144, 255
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %6) #6
  %146 = add nsw i64 %12, 5
  call fastcc void @fill(ptr noundef nonnull %6, i32 noundef %0, i64 noundef %146)
  %147 = load i32, ptr %6, align 8, !tbaa !9
  %148 = and i32 %147, 255
  %149 = getelementptr inbounds %struct.pair, ptr %6, i32 0, i32 1
  %150 = load i64, ptr %149, align 8, !tbaa !12
  %151 = trunc i64 %150 to i32
  %152 = and i32 %151, 255
  %153 = getelementptr inbounds %struct.pair, ptr %6, i32 0, i32 2
  %154 = load i64, ptr %153, align 8, !tbaa !13
  %155 = lshr i64 %154, 56
  %156 = trunc i64 %155 to i32
  %157 = tail call fastcc i32 @classify(i64 noundef %12), !range !14
  %158 = tail call fastcc i32 @classify(i64 noundef 0), !range !14
  %159 = tail call fastcc i32 @classify(i64 noundef -1), !range !14
  %160 = tail call fastcc i32 @classify(i64 noundef 1099511627776), !range !14
  %161 = tail call fastcc i32 @classify(i64 noundef 123456789012345), !range !14
  %162 = tail call i64 @llvm.ctlz.i64(i64 %117, i1 true), !range !15
  %163 = trunc i64 %162 to i32
  %164 = or i64 %12, 1125899906842624
  %165 = tail call i64 @llvm.cttz.i64(i64 %164, i1 true), !range !16
  %166 = trunc i64 %165 to i32
  %167 = xor i64 %12, 255
  %168 = tail call i64 @llvm.ctpop.i64(i64 %167), !range !17
  %169 = trunc i64 %168 to i32
  %170 = tail call i64 @llvm.bitreverse.i64(i64 %12)
  %171 = lshr i64 %170, 60
  %172 = trunc i64 %171 to i32
  %173 = lshr i32 %0, 24
  %174 = add nsw i64 %12, -1000
  %175 = tail call i64 @llvm.abs.i64(i64 %174, i1 true)
  %176 = trunc i64 %175 to i32
  %177 = and i32 %176, 255
  %178 = add nuw nsw i32 %173, %94
  %179 = add nuw nsw i32 %178, %166
  %.5 = add nuw nsw i32 %179, %163
  %.6 = add nuw nsw i32 %.5, %169
  %.7 = add nuw nsw i32 %.6, %93
  %.8 = add nuw nsw i32 %.7, %172
  %180 = add nuw nsw i32 %.8, %177
  %181 = add nuw nsw i32 %180, %.5.v
  %182 = add nuw nsw i32 %181, %.6.v
  %183 = add nuw nsw i32 %182, %.7.v
  %184 = add nuw nsw i32 %183, %.8.v
  %185 = add nuw nsw i32 %184, %122
  %186 = add nuw nsw i32 %185, %125
  %187 = add nuw nsw i32 %186, %130
  %188 = add nuw nsw i32 %187, %133
  %189 = add nuw nsw i32 %188, %141
  %190 = add nuw nsw i32 %189, %145
  %191 = add nuw nsw i32 %190, %148
  %192 = add nuw nsw i32 %191, %157
  %193 = add nuw nsw i32 %192, %152
  %194 = add nuw nsw i32 %193, %158
  %195 = add nuw nsw i32 %194, %156
  %196 = add nuw nsw i32 %195, %.4
  %197 = add nuw nsw i32 %196, %159
  %198 = add nuw nsw i32 %197, %160
  %199 = add nuw nsw i32 %198, %161
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %6) #6
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #6
  ret i32 %199
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #1

; Function Attrs: mustprogress nofree noinline norecurse nosync nounwind willreturn memory(none)
define internal fastcc i64 @sum_squares(i64 noundef %0) unnamed_addr #2 {
  %.not8 = icmp slt i64 %0, 1
  br i1 %.not8, label %._crit_edge, label %.lr.ph.preheader

.lr.ph.preheader:                                 ; preds = %1
  %2 = add nsw i64 %0, -1
  %3 = zext nneg i64 %2 to i65
  %4 = add nsw i64 %0, -2
  %5 = zext i64 %4 to i65
  %6 = mul i65 %3, %5
  %7 = add nsw i64 %0, -3
  %8 = zext i64 %7 to i65
  %9 = mul i65 %6, %8
  %10 = lshr i65 %9, 1
  %11 = trunc i65 %10 to i64
  %12 = mul i64 %11, 6148914691236517206
  %13 = lshr i65 %6, 1
  %14 = trunc i65 %13 to i64
  %15 = mul i64 %14, 5
  %16 = add i64 %12, %15
  %17 = shl i64 %0, 2
  %18 = add i64 %16, %17
  %19 = add i64 %18, -3
  br label %._crit_edge

._crit_edge:                                      ; preds = %.lr.ph.preheader, %1
  %.07.lcssa = phi i64 [ 0, %1 ], [ %19, %.lr.ph.preheader ]
  ret i64 %.07.lcssa
}

; Function Attrs: mustprogress nofree noinline norecurse nosync nounwind willreturn memory(argmem: write)
define internal fastcc i64 @mul_parts(i64 noundef %0, i64 noundef %1, ptr nocapture noundef writeonly %2) unnamed_addr #3 {
  %4 = mul i64 %1, %0
  store i64 %4, ptr %2, align 8, !tbaa !5
  %5 = lshr i64 %0, 32
  %6 = lshr i64 %1, 32
  %7 = mul nuw i64 %6, %5
  ret i64 %7
}

; Function Attrs: mustprogress nofree noinline norecurse nosync nounwind willreturn memory(none)
define internal fastcc i64 @rotl64(i64 noundef %0, i32 noundef %1) unnamed_addr #2 {
  %3 = and i32 %1, 63
  %.not = icmp eq i32 %3, 0
  br i1 %.not, label %11, label %4

4:                                                ; preds = %2
  %5 = zext nneg i32 %3 to i64
  %6 = shl i64 %0, %5
  %7 = sub nuw nsw i32 64, %3
  %8 = zext nneg i32 %7 to i64
  %9 = lshr i64 %0, %8
  %10 = or i64 %9, %6
  br label %11

11:                                               ; preds = %2, %4
  %12 = phi i64 [ %10, %4 ], [ %0, %2 ]
  ret i64 %12
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64) #4

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.umul.with.overflow.i64(i64, i64) #4

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64) #4

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.usub.with.overflow.i64(i64, i64) #4

; Function Attrs: mustprogress nofree noinline norecurse nosync nounwind willreturn memory(argmem: write)
define internal fastcc void @fill(ptr nocapture noundef writeonly %0, i32 noundef %1, i64 noundef %2) unnamed_addr #3 {
  store i32 %1, ptr %0, align 8, !tbaa !9
  %4 = getelementptr inbounds %struct.pair, ptr %0, i32 0, i32 1
  store i64 %2, ptr %4, align 8, !tbaa !12
  %5 = xor i64 %2, -6510615555426900571
  %6 = getelementptr inbounds %struct.pair, ptr %0, i32 0, i32 2
  store i64 %5, ptr %6, align 8, !tbaa !13
  ret void
}

; Function Attrs: mustprogress nofree noinline norecurse nosync nounwind willreturn memory(none)
define internal fastcc noundef i32 @classify(i64 noundef %0) unnamed_addr #2 {
  switch i64 %0, label %5 [
    i64 0, label %6
    i64 -1, label %2
    i64 1099511627776, label %3
    i64 123456789012345, label %4
  ]

2:                                                ; preds = %1
  br label %6

3:                                                ; preds = %1
  br label %6

4:                                                ; preds = %1
  br label %6

5:                                                ; preds = %1
  br label %6

6:                                                ; preds = %1, %5, %4, %3, %2
  %.0 = phi i32 [ 5, %5 ], [ 4, %4 ], [ 3, %3 ], [ 2, %2 ], [ 1, %1 ]
  ret i32 %.0
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.ctlz.i64(i64, i1 immarg) #4

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.cttz.i64(i64, i1 immarg) #4

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.ctpop.i64(i64) #4

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.bitreverse.i64(i64) #4

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.abs.i64(i64, i1 immarg) #4

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #1

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smin.i64(i64, i64) #5

attributes #0 = { mustprogress nofree norecurse nosync nounwind willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #1 = { mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #2 = { mustprogress nofree noinline norecurse nosync nounwind willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #3 = { mustprogress nofree noinline norecurse nosync nounwind willreturn memory(argmem: write) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="generic-rv32" "target-features"="+32bit,+m,+relax,-a,-c,-d,-e,-experimental-zacas,-experimental-zcmop,-experimental-zfbfmin,-experimental-zicfilp,-experimental-zicfiss,-experimental-zimop,-experimental-ztso,-experimental-zvfbfmin,-experimental-zvfbfwma,-f,-h,-smaia,-smepmp,-ssaia,-svinval,-svnapot,-svpbmt,-v,-xcvalu,-xcvbi,-xcvbitmanip,-xcvelw,-xcvmac,-xcvmem,-xcvsimd,-xsfvcp,-xsfvfnrclipxfqf,-xsfvfwmaccqqq,-xsfvqmaccdod,-xsfvqmaccqoq,-xtheadba,-xtheadbb,-xtheadbs,-xtheadcmo,-xtheadcondmov,-xtheadfmemidx,-xtheadmac,-xtheadmemidx,-xtheadmempair,-xtheadsync,-xtheadvdot,-xventanacondops,-za128rs,-za64rs,-zawrs,-zba,-zbb,-zbc,-zbkb,-zbkc,-zbkx,-zbs,-zca,-zcb,-zcd,-zce,-zcf,-zcmp,-zcmt,-zdinx,-zfa,-zfh,-zfhmin,-zfinx,-zhinx,-zhinxmin,-zic64b,-zicbom,-zicbop,-zicboz,-ziccamoa,-ziccif,-zicclsm,-ziccrse,-zicntr,-zicond,-zicsr,-zifencei,-zihintntl,-zihintpause,-zihpm,-zk,-zkn,-zknd,-zkne,-zknh,-zkr,-zks,-zksed,-zksh,-zkt,-zmmul,-zvbb,-zvbc,-zve32f,-zve32x,-zve64d,-zve64f,-zve64x,-zvfh,-zvfhmin,-zvkb,-zvkg,-zvkn,-zvknc,-zvkned,-zvkng,-zvknha,-zvknhb,-zvks,-zvksc,-zvksed,-zvksg,-zvksh,-zvkt,-zvl1024b,-zvl128b,-zvl16384b,-zvl2048b,-zvl256b,-zvl32768b,-zvl32b,-zvl4096b,-zvl512b,-zvl64b,-zvl65536b,-zvl8192b" }
attributes #4 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #5 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #6 = { nounwind }

!llvm.module.flags = !{!0, !1, !2}
!llvm.ident = !{!3}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 1, !"target-abi", !"ilp32"}
!2 = !{i32 8, !"SmallDataLimit", i32 8}
!3 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
!4 = !{i64 0, i64 -8589934590}
!5 = !{!6, !6, i64 0}
!6 = !{!"long long", !7, i64 0}
!7 = !{!"omnipotent char", !8, i64 0}
!8 = !{!"Simple C/C++ TBAA"}
!9 = !{!10, !11, i64 0}
!10 = !{!"pair", !11, i64 0, !6, i64 8, !6, i64 16}
!11 = !{!"int", !7, i64 0}
!12 = !{!10, !6, i64 8}
!13 = !{!10, !6, i64 16}
!14 = !{i32 1, i32 6}
!15 = !{i64 0, i64 64}
!16 = !{i64 0, i64 51}
!17 = !{i64 0, i64 65}
