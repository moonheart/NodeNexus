import React from 'react';
import { Link as RouterLink } from 'react-router-dom';
import { Button, Container, Typography } from '@mui/material';

const NotFoundPage: React.FC = () => {
  return (
    <Container component="main" maxWidth="sm" sx={{ textAlign: 'center', mt: 8 }}>
      <Typography variant="h1" component="h1" gutterBottom>
        404
      </Typography>
      <Typography variant="h5" component="h2" gutterBottom>
        哎呀！页面未找到
      </Typography>
      <Typography variant="body1" sx={{ mb: 4 }}>
        您要查找的页面不存在，或者已被移动。
      </Typography>
      <Button component={RouterLink} to="/" variant="contained" color="primary">
        返回首页
      </Button>
    </Container>
  );
};

export default NotFoundPage;