import React from 'react';
import { Skeleton } from "@/components/ui/skeleton";
import { Card, CardHeader, CardContent, CardFooter } from "@/components/ui/card";

const TagSkeletonCard: React.FC = () => {
  return (
    <Card>
      {/* Header */}
      <CardHeader>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Skeleton className="w-5 h-5" />
            <Skeleton className="w-24 h-6" />
          </div>
          <Skeleton className="w-12 h-6" />
        </div>
      </CardHeader>

      {/* Body */}
      <CardContent className="space-y-3">
        <div className="flex items-center justify-between">
          <Skeleton className="w-20 h-4" />
          <Skeleton className="w-8 h-4" />
        </div>
        <div className="flex items-center justify-between">
          <Skeleton className="w-24 h-4" />
          <Skeleton className="w-12 h-4" />
        </div>
      </CardContent>

      {/* Footer */}
      <CardFooter className="flex items-center justify-between">
        <Skeleton className="w-8 h-8 rounded-full" />
        <div className="flex items-center gap-2">
          <Skeleton className="w-8 h-8" />
          <Skeleton className="w-8 h-8" />
        </div>
      </CardFooter>
    </Card>
  );
};

export default TagSkeletonCard;