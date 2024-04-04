import {
  CaretRightOutlined,
  FastBackwardOutlined,
  FastForwardOutlined,
  PauseOutlined
} from '@ant-design/icons';
import { Button, Col, Flex, Row, Slider } from 'antd';

function App() {
  return (
    <>
      <Row gutter={[8, 8]} align="middle">
        <Col flex="none">
          <Flex gap="small" wrap="wrap">
            <Button type="primary" shape="circle" size="large" icon={<FastBackwardOutlined />} />
            <Button type="primary" shape="circle" size="large" icon={<PauseOutlined />} />
            <Button type="primary" shape="circle" size="large" icon={<CaretRightOutlined />} />
            <Button type="primary" shape="circle" size="large" icon={<FastForwardOutlined />} />
          </Flex>
        </Col>
        <Col flex="auto">
          <Slider />
        </Col>
      </Row>
    </>
  );
}

export default App;
