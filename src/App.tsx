import {
  CaretRightOutlined,
  PauseOutlined,
  StepBackwardOutlined,
  StepForwardOutlined,
  StopOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { Button, Col, Flex, Row, Slider } from 'antd';

function App() {
  return (
    <>
      <Row gutter={[8, 8]} align="middle">
        <Col flex="none">
          <Flex gap="small" wrap="wrap">
            <Button type="primary" shape="circle" size="large" icon={<StepBackwardOutlined />} />
            <Button
              type="primary"
              shape="circle"
              size="large"
              icon={<StopOutlined />}
              onClick={(_) => invoke('stop')}
            />
            <Button
              type="primary"
              shape="circle"
              size="large"
              icon={<PauseOutlined />}
              onClick={(_) => invoke('pause')}
            />
            <Button
              type="primary"
              shape="circle"
              size="large"
              icon={<CaretRightOutlined />}
              onClick={(_) =>
                invoke('play', {
                  uri: 'file:///home/misha/Musique/Boy Meets Girl/Boy Meets Girl - Waiting For A Star To Fall.mp3'
                })
              }
            />
            <Button type="primary" shape="circle" size="large" icon={<StepForwardOutlined />} />
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
